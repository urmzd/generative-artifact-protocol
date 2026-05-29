package main

import (
	"context"
	"crypto/rand"
	"encoding/base64"
	"encoding/json"
	"errors"
	"fmt"
	"log"
	"math"
	mathrand "math/rand"
	"net/http"
	"os"
	"os/signal"
	"sort"
	"strconv"
	"strings"
	"sync"
	"time"
)

type URL struct {
	Code       string    `json:"code"`
	LongURL    string    `json:"long_url"`
	ShortURL   string    `json:"short_url"`
	CreatedAt  time.Time `json:"created_at"`
	Clicks     int64     `json:"clicks"`
	LastAccess time.Time `json:"last_access,omitempty"`
	DeletedAt  time.Time `json:"deleted_at,omitempty"`
	Deleted    bool      `json:"deleted"`
}

type CreateRequest struct {
	URL string `json:"url"`
}

type StatsResponse struct {
	Code       string    `json:"code"`
	LongURL    string    `json:"long_url"`
	ShortURL   string    `json:"short_url"`
	CreatedAt  time.Time `json:"created_at"`
	Clicks     int64     `json:"clicks"`
	LastAccess time.Time `json:"last_access,omitempty"`
}

type Store struct {
	mu        sync.RWMutex
	urls      map[string]*URL
	byLongURL map[string]string
}

func NewStore() *Store {
	return &Store{
		urls:      make(map[string]*URL),
		byLongURL: make(map[string]string),
	}
}

func (s *Store) Create(longURL, shortURL, code string) *URL {
	s.mu.Lock()
	defer s.mu.Unlock()

	if existingCode, ok := s.byLongURL[longURL]; ok {
		if existing, ok := s.urls[existingCode]; ok && !existing.Deleted {
			return existing
		}
	}

	u := &URL{
		Code:      code,
		LongURL:   longURL,
		ShortURL:  shortURL,
		CreatedAt: time.Now().UTC(),
	}
	s.urls[code] = u
	s.byLongURL[longURL] = code
	return u
}

func (s *Store) Get(code string) (*URL, bool) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	u, ok := s.urls[code]
	if !ok || u.Deleted {
		return nil, false
	}
	cp := *u
	return &cp, true
}

func (s *Store) GetForUpdate(code string) (*URL, bool) {
	s.mu.Lock()
	defer s.mu.Unlock()
	u, ok := s.urls[code]
	if !ok || u.Deleted {
		return nil, false
	}
	u.Clicks++
	u.LastAccess = time.Now().UTC()
	cp := *u
	return &cp, true
}

func (s *Store) Delete(code string) bool {
	s.mu.Lock()
	defer s.mu.Unlock()
	u, ok := s.urls[code]
	if !ok || u.Deleted {
		return false
	}
	u.Deleted = true
	u.DeletedAt = time.Now().UTC()
	delete(s.byLongURL, u.LongURL)
	return true
}

func (s *Store) List() []URL {
	s.mu.RLock()
	defer s.mu.RUnlock()
	out := make([]URL, 0, len(s.urls))
	for _, u := range s.urls {
		if u.Deleted {
			continue
		}
		out = append(out, *u)
	}
	return out
}

func (s *Store) Stats(code string) (*StatsResponse, bool) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	u, ok := s.urls[code]
	if !ok || u.Deleted {
		return nil, false
	}
	return &StatsResponse{
		Code:       u.Code,
		LongURL:    u.LongURL,
		ShortURL:   u.ShortURL,
		CreatedAt:  u.CreatedAt,
		Clicks:     u.Clicks,
		LastAccess: u.LastAccess,
	}, true
}

func (s *Store) Top(limit int) []URL {
	s.mu.RLock()
	defer s.mu.RUnlock()

	out := make([]URL, 0, len(s.urls))
	for _, u := range s.urls {
		if u.Deleted {
			continue
		}
		out = append(out, *u)
	}

	sort.Slice(out, func(i, j int) bool {
		if out[i].Clicks == out[j].Clicks {
			return out[i].CreatedAt.Before(out[j].CreatedAt)
		}
		return out[i].Clicks > out[j].Clicks
	})

	if limit <= 0 || limit > len(out) {
		return out
	}
	return out[:limit]
}

type tokenBucket struct {
	capacity float64
	tokens   float64
	rate     float64
	last     time.Time
	mu       sync.Mutex
}

func newTokenBucket(rate, burst int) *tokenBucket {
	return &tokenBucket{
		capacity: float64(burst),
		tokens:   float64(burst),
		rate:     float64(rate),
		last:     time.Now(),
	}
}

func (b *tokenBucket) allow() bool {
	b.mu.Lock()
	defer b.mu.Unlock()

	now := time.Now()
	elapsed := now.Sub(b.last).Seconds()
	b.last = now
	b.tokens = math.Min(b.capacity, b.tokens+elapsed*b.rate)
	if b.tokens >= 1 {
		b.tokens -= 1
		return true
	}
	return false
}

type App struct {
	store      *Store
	baseURL    string
	codeLength int
	bucket     *tokenBucket
}

func (a *App) CreateShortURL(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		writeJSONError(w, http.StatusMethodNotAllowed, "method not allowed")
		return
	}

	var req CreateRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeJSONError(w, http.StatusBadRequest, "invalid json")
		return
	}
	req.URL = strings.TrimSpace(req.URL)
	if req.URL == "" {
		writeJSONError(w, http.StatusBadRequest, "url is required")
		return
	}
	if !isValidURL(req.URL) {
		writeJSONError(w, http.StatusBadRequest, "invalid url")
		return
	}

	code := a.generateCode()
	shortURL := strings.TrimRight(a.baseURL, "/") + "/" + code
	u := a.store.Create(req.URL, shortURL, code)

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusCreated)
	_ = json.NewEncoder(w).Encode(u)
}

func (a *App) RedirectURL(w http.ResponseWriter, r *http.Request) {
	code := strings.TrimPrefix(r.URL.Path, "/")
	if code == "" || code == "stats" || code == "urls" {
		writeJSONError(w, http.StatusNotFound, "not found")
		return
	}
	if idx := strings.IndexByte(code, '/'); idx >= 0 {
		code = code[:idx]
	}

	u, ok := a.store.GetForUpdate(code)
	if !ok {
		writeJSONError(w, http.StatusNotFound, "short url not found")
		return
	}

	http.Redirect(w, r, u.LongURL, http.StatusFound)
}

func (a *App) GetStats(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		writeJSONError(w, http.StatusMethodNotAllowed, "method not allowed")
		return
	}

	code := strings.TrimPrefix(r.URL.Path, "/stats/")
	if code == "" || code == r.URL.Path {
		writeJSONError(w, http.StatusNotFound, "not found")
		return
	}

	stats, ok := a.store.Stats(code)
	if !ok {
		writeJSONError(w, http.StatusNotFound, "short url not found")
		return
	}

	w.Header().Set("Content-Type", "application/json")
	_ = json.NewEncoder(w).Encode(stats)
}

func (a *App) ListURLs(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		writeJSONError(w, http.StatusMethodNotAllowed, "method not allowed")
		return
	}

	urls := a.store.List()
	w.Header().Set("Content-Type", "application/json")
	_ = json.NewEncoder(w).Encode(urls)
}

func (a *App) GetTopURLs(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		writeJSONError(w, http.StatusMethodNotAllowed, "method not allowed")
		return
	}

	limit := 10
	if raw := strings.TrimSpace(r.URL.Query().Get("limit")); raw != "" {
		n, err := strconv.Atoi(raw)
		if err != nil || n < 0 {
			writeJSONError(w, http.StatusBadRequest, "invalid limit")
			return
		}
		limit = n
	}

	urls := a.store.Top(limit)
	w.Header().Set("Content-Type", "application/json")
	_ = json.NewEncoder(w).Encode(urls)
}

func (a *App) DeleteURL(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodDelete {
		writeJSONError(w, http.StatusMethodNotAllowed, "method not allowed")
		return
	}

	code := strings.TrimPrefix(r.URL.Path, "/")
	if code == "" {
		writeJSONError(w, http.StatusNotFound, "not found")
		return
	}
	if idx := strings.IndexByte(code, '/'); idx >= 0 {
		code = code[:idx]
	}
	if code == "stats" || code == "urls" {
		writeJSONError(w, http.StatusNotFound, "not found")
		return
	}

	if !a.store.Delete(code) {
		writeJSONError(w, http.StatusNotFound, "short url not found")
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

func (a *App) generateCode() string {
	for i := 0; i < 10; i++ {
		code := randomCode(a.codeLength)
		if _, ok := a.store.Get(code); !ok {
			return code
		}
	}
	return fmt.Sprintf("%d", mathrand.Int63())
}

func randomCode(n int) string {
	if n <= 0 {
		n = 6
	}
	b := make([]byte, n)
	_, _ = rand.Read(b)
	s := base64.RawURLEncoding.EncodeToString(b)
	if len(s) > n {
		return s[:n]
	}
	for len(s) < n {
		s += "a"
	}
	return s
}

func isValidURL(raw string) bool {
	u, err := http.NewRequest(http.MethodGet, raw, nil)
	return err == nil && u.URL.Scheme != "" && u.URL.Host != ""
}

type responseWriter struct {
	http.ResponseWriter
	status int
	bytes  int64
}

func (w *responseWriter) WriteHeader(statusCode int) {
	w.status = statusCode
	w.ResponseWriter.WriteHeader(statusCode)
}

func (w *responseWriter) Write(p []byte) (int, error) {
	if w.status == 0 {
		w.status = http.StatusOK
	}
	n, err := w.ResponseWriter.Write(p)
	w.bytes += int64(n)
	return n, err
}

func requestIDMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		id := r.Header.Get("X-Request-ID")
		if id == "" {
			id = randomCode(12)
		}
		w.Header().Set("X-Request-ID", id)
		ctx := context.WithValue(r.Context(), requestIDKey{}, id)
		next.ServeHTTP(w, r.WithContext(ctx))
	})
}

func loggingMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		start := time.Now()
		rw := &responseWriter{ResponseWriter: w}
		next.ServeHTTP(rw, r)
		if rw.status == 0 {
			rw.status = http.StatusOK
		}
		id, _ := r.Context().Value(requestIDKey{}).(string)
		log.Printf("request_id=%s method=%s path=%s status=%d bytes=%d duration=%s remote=%s",
			id, r.Method, r.URL.Path, rw.status, rw.bytes, time.Since(start), r.RemoteAddr)
	})
}

func corsMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Access-Control-Allow-Origin", "*")
		w.Header().Set("Access-Control-Allow-Methods", "GET, POST, DELETE, OPTIONS")
		w.Header().Set("Access-Control-Allow-Headers", "Content-Type, X-Request-ID")
		if r.Method == http.MethodOptions {
			w.WriteHeader(http.StatusNoContent)
			return
		}
		next.ServeHTTP(w, r)
	})
}

func rateLimitMiddleware(bucket *tokenBucket) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			if !bucket.allow() {
				writeJSONError(w, http.StatusTooManyRequests, "rate limit exceeded")
				return
			}
			next.ServeHTTP(w, r)
		})
	}
}

type requestIDKey struct{}

func writeJSONError(w http.ResponseWriter, status int, msg string) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(map[string]string{"error": msg})
}

func envInt(name string, def int) int {
	v := strings.TrimSpace(os.Getenv(name))
	if v == "" {
		return def
	}
	n, err := strconv.Atoi(v)
	if err != nil || n <= 0 {
		return def
	}
	return n
}

func envString(name, def string) string {
	v := strings.TrimSpace(os.Getenv(name))
	if v == "" {
		return def
	}
	return v
}

func main() {
	addr := envString("ADDR", ":8080")
	baseURL := envString("BASE_URL", "http://localhost"+addr)
	codeLen := envInt("CODE_LENGTH", 6)
	rate := envInt("RATE_LIMIT_RPS", 50)
	burst := envInt("RATE_LIMIT_BURST", 100)

	store := NewStore()
	app := &App{
		store:      store,
		baseURL:    baseURL,
		codeLength: codeLen,
		bucket:     newTokenBucket(rate, burst),
	}

	mux := http.NewServeMux()
	mux.HandleFunc("/urls/top", app.GetTopURLs)
	mux.HandleFunc("/urls", app.ListURLs)
	mux.HandleFunc("/stats/", app.GetStats)
	mux.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		switch {
		case r.URL.Path == "/" && r.Method == http.MethodPost:
			app.CreateShortURL(w, r)
		case r.Method == http.MethodPost && r.URL.Path == "/shorten":
			app.CreateShortURL(w, r)
		case r.Method == http.MethodDelete:
			app.DeleteURL(w, r)
		case r.Method == http.MethodGet:
			app.RedirectURL(w, r)
		default:
			writeJSONError(w, http.StatusNotFound, "not found")
		}
	})

	handler := requestIDMiddleware(loggingMiddleware(corsMiddleware(rateLimitMiddleware(app.bucket)(mux))))

	srv := &http.Server{
		Addr:    addr,
		Handler: handler,
	}

	go func() {
		log.Printf("server starting addr=%s base_url=%s", addr, baseURL)
		if err := srv.ListenAndServe(); err != nil && !errors.Is(err, http.ErrServerClosed) {
			log.Fatalf("server error: %v", err)
		}
	}()

	stop := make(chan os.Signal, 1)
	signal.Notify(stop, os.Interrupt)

	<-stop
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	_ = srv.Shutdown(ctx)
}