<gap:target id="file">
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
	"net/http"
	"os"
	"strconv"
	"strings"
	"sync"
	"sync/atomic"
	"syscall"
	"time"
)

<gap:target id="config-block">
type Config struct {
	<gap:target id="server-address">Addr</gap:target>            string
	<gap:target id="shutdown-timeout">ShutdownTimeout</gap:target> time.Duration
	<gap:target id="rate-limit-rps">RateLimitRPS</gap:target>      float64
	<gap:target id="rate-limit-burst">RateLimitBurst</gap:target>  float64
}

func loadConfig() Config {
	return Config{
		Addr:            getEnv("ADDR", ":8080"),
		ShutdownTimeout: getEnvDuration("SHUTDOWN_TIMEOUT", 10*time.Second),
		RateLimitRPS:    getEnvFloat("RATE_LIMIT_RPS", 5),
		RateLimitBurst:  getEnvFloat("RATE_LIMIT_BURST", 10),
	}
}

func getEnv(key, fallback string) string {
	if v := strings.TrimSpace(os.Getenv(key)); v != "" {
		return v
	}
	return fallback
}

func getEnvDuration(key string, fallback time.Duration) time.Duration {
	v := strings.TrimSpace(os.Getenv(key))
	if v == "" {
		return fallback
	}
	d, err := time.ParseDuration(v)
	if err != nil {
		return fallback
	}
	return d
}

func getEnvFloat(key string, fallback float64) float64 {
	v := strings.TrimSpace(os.Getenv(key))
	if v == "" {
		return fallback
	}
	f, err := strconv.ParseFloat(v, 64)
	if err != nil || math.IsNaN(f) || math.IsInf(f, 0) {
		return fallback
	}
	return f
}
</gap:target>

<gap:target id="model-block">
type URL struct {
	<gap:target id="url-code">Code</gap:target>        string    `json:"code"`
	<gap:target id="url-original">OriginalURL</gap:target> string    `json:"original_url"`
	<gap:target id="url-short">ShortURL</gap:target>    string    `json:"short_url"`
	<gap:target id="url-created">CreatedAt</gap:target>  time.Time `json:"created_at"`
	<gap:target id="url-clicks">Clicks</gap:target>     int64     `json:"clicks"`
}

type CreateRequest struct {
	<gap:target id="create-original-url">OriginalURL</gap:target> string `json:"original_url"`
}

type StatsResponse struct {
	<gap:target id="stats-code">Code</gap:target>        string    `json:"code"`
	<gap:target id="stats-original">OriginalURL</gap:target> string    `json:"original_url"`
	<gap:target id="stats-short">ShortURL</gap:target>    string    `json:"short_url"`
	<gap:target id="stats-created">CreatedAt</gap:target>  time.Time `json:"created_at"`
	<gap:target id="stats-clicks">Clicks</gap:target>     int64     `json:"clicks"`
}
</gap:target>

<gap:target id="store-block">
type Store struct {
	mu   sync.RWMutex
	urls map[string]*URL
}

func NewStore() *Store {
	return &Store{urls: make(map[string]*URL)}
}

func (s *Store) Create(originalURL, baseURL string) *URL {
	code := generateCode(8)
	now := time.Now().UTC()

	u := &URL{
		Code:        code,
		OriginalURL: originalURL,
		ShortURL:    strings.TrimRight(baseURL, "/") + "/" + code,
		CreatedAt:    now,
		Clicks:      0,
	}

	s.mu.Lock()
	s.urls[code] = u
	s.mu.Unlock()

	return u
}

func (s *Store) Get(code string) (*URL, bool) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	u, ok := s.urls[code]
	if !ok {
		return nil, false
	}
	copyURL := *u
	return &copyURL, true
}

func (s *Store) IncrementClicks(code string) (*URL, bool) {
	s.mu.Lock()
	defer s.mu.Unlock()
	u, ok := s.urls[code]
	if !ok {
		return nil, false
	}
	u.Clicks++
	copyURL := *u
	return &copyURL, true
}

func (s *Store) Delete(code string) bool {
	s.mu.Lock()
	defer s.mu.Unlock()
	if _, ok := s.urls[code]; !ok {
		return false
	}
	delete(s.urls, code)
	return true
}

func (s *Store) List() []URL {
	s.mu.RLock()
	defer s.mu.RUnlock()
	out := make([]URL, 0, len(s.urls))
	for _, u := range s.urls {
		out = append(out, *u)
	}
	return out
}
</gap:target>

<gap:target id="helpers-block">
func generateCode(n int) string {
	b := make([]byte, n)
	if _, err := rand.Read(b); err != nil {
		return fmt.Sprintf("%d", time.Now().UnixNano())
	}
	return strings.TrimRight(base64.URLEncoding.EncodeToString(b), "=")
}

func writeJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(v)
}

func writeError(w http.ResponseWriter, status int, msg string) {
	writeJSON(w, status, map[string]string{"error": msg})
}

func getCodeFromPath(path string) string {
	parts := strings.Split(strings.Trim(path, "/"), "/")
	if len(parts) == 0 {
		return ""
	}
	return parts[len(parts)-1]
}
</gap:target>

<gap:target id="rate-limiter-block">
type tokenBucket struct {
	tokens float64
	last   time.Time
	rps    float64
	burst  float64
	mu     sync.Mutex
}

func newTokenBucket(rps, burst float64) *tokenBucket {
	return &tokenBucket{
		tokens: burst,
		last:   time.Now(),
		rps:    rps,
		burst:  burst,
	}
}

func (tb *tokenBucket) allow() bool {
	tb.mu.Lock()
	defer tb.mu.Unlock()

	now := time.Now()
	elapsed := now.Sub(tb.last).Seconds()
	tb.last = now

	tb.tokens += elapsed * tb.rps
	if tb.tokens > tb.burst {
		tb.tokens = tb.burst
	}
	if tb.tokens < 1 {
		return false
	}
	tb.tokens -= 1
	return true
}
</gap:target>

<gap:target id="middleware-block">
func requestIDMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		id := r.Header.Get("X-Request-ID")
		if id == "" {
			id = generateCode(12)
		}
		w.Header().Set("X-Request-ID", id)
		next.ServeHTTP(w, r.WithContext(context.WithValue(r.Context(), requestIDKey{}, id)))
	})
}

func loggingMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		start := time.Now()
		next.ServeHTTP(w, r)
		requestID, _ := r.Context().Value(requestIDKey{}).(string)
		log.Printf("request_id=%s method=%s path=%s duration=%s", requestID, r.Method, r.URL.Path, time.Since(start))
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

func rateLimitMiddleware(tb *tokenBucket) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			if !tb.allow() {
				writeError(w, http.StatusTooManyRequests, "rate limit exceeded")
				return
			}
			next.ServeHTTP(w, r)
		})
	}
}

type requestIDKey struct{}
</gap:target>

<gap:target id="handler-block">
type Handler struct {
	store   *Store
	baseURL string
}

func (h *Handler) CreateShortURL(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		writeError(w, http.StatusMethodNotAllowed, "method not allowed")
		return
	}

	var req CreateRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeError(w, http.StatusBadRequest, "invalid JSON")
		return
	}
	if strings.TrimSpace(req.OriginalURL) == "" {
		writeError(w, http.StatusBadRequest, "original_url is required")
		return
	}

	u := h.store.Create(req.OriginalURL, h.baseURL)
	writeJSON(w, http.StatusCreated, u)
}

func (h *Handler) RedirectURL(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		writeError(w, http.StatusMethodNotAllowed, "method not allowed")
		return
	}
	code := getCodeFromPath(r.URL.Path)
	if code == "" {
		writeError(w, http.StatusNotFound, "not found")
		return
	}

	u, ok := h.store.IncrementClicks(code)
	if !ok {
		writeError(w, http.StatusNotFound, "not found")
		return
	}
	http.Redirect(w, r, u.OriginalURL, http.StatusFound)
}

func (h *Handler) GetStats(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		writeError(w, http.StatusMethodNotAllowed, "method not allowed")
		return
	}
	code := getCodeFromPath(r.URL.Path)
	u, ok := h.store.Get(code)
	if !ok {
		writeError(w, http.StatusNotFound, "not found")
		return
	}
	writeJSON(w, http.StatusOK, StatsResponse{
		Code:        u.Code,
		OriginalURL: u.OriginalURL,
		ShortURL:    u.ShortURL,
		CreatedAt:   u.CreatedAt,
		Clicks:      u.Clicks,
	})
}

func (h *Handler) ListURLs(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		writeError(w, http.StatusMethodNotAllowed, "method not allowed")
		return
	}
	writeJSON(w, http.StatusOK, h.store.List())
}

func (h *Handler) DeleteURL(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodDelete {
		writeError(w, http.StatusMethodNotAllowed, "method not allowed")
		return
	}
	code := getCodeFromPath(r.URL.Path)
	if code == "" {
		writeError(w, http.StatusNotFound, "not found")
		return
	}
	if !h.store.Delete(code) {
		writeError(w, http.StatusNotFound, "not found")
		return
	}
	w.WriteHeader(http.StatusNoContent)
}
</gap:target>

<gap:target id="routing-block">
func main() {
	cfg := loadConfig()
	store := NewStore()
	handler := &Handler{
		store:   store,
		baseURL: baseURLFromAddr(cfg.Addr),
	}

	tb := newTokenBucket(cfg.RateLimitRPS, cfg.RateLimitBurst)

	mux := http.NewServeMux()
	mux.HandleFunc("/shorten", handler.CreateShortURL)
	mux.HandleFunc("/stats/", handler.GetStats)
	mux.HandleFunc("/urls", handler.ListURLs)
	mux.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		switch r.Method {
		case http.MethodGet:
			if strings.HasPrefix(r.URL.Path, "/stats/") {
				handler.GetStats(w, r)
				return
			}
			if r.URL.Path == "/urls" {
				handler.ListURLs(w, r)
				return
			}
			handler.RedirectURL(w, r)
		case http.MethodDelete:
			handler.DeleteURL(w, r)
		case http.MethodPost:
			if r.URL.Path == "/shorten" {
				handler.CreateShortURL(w, r)
				return
			}
			writeError(w, http.StatusNotFound, "not found")
		default:
			writeError(w, http.StatusNotFound, "not found")
		}
	})

	var finalHandler http.Handler = mux
	finalHandler = requestIDMiddleware(finalHandler)
	finalHandler = loggingMiddleware(finalHandler)
	finalHandler = corsMiddleware(finalHandler)
	finalHandler = rateLimitMiddleware(tb)(finalHandler)

	server := &http.Server{
		Addr:    cfg.Addr,
		Handler: finalHandler,
	}

	go func() {
		log.Printf("server starting on %s", cfg.Addr)
		if err := server.ListenAndServe(); err != nil && !errors.Is(err, http.ErrServerClosed) {
			log.Fatalf("server error: %v", err)
		}
	}()

	stop := make(chan os.Signal, 1)
	signalNotify(stop, syscall.SIGINT, syscall.SIGTERM)
	<-stop

	ctx, cancel := context.WithTimeout(context.Background(), cfg.ShutdownTimeout)
	defer cancel()

	if err := server.Shutdown(ctx); err != nil {
		log.Printf("shutdown error: %v", err)
	}
	log.Println("server stopped")
}

func baseURLFromAddr(addr string) string {
	if strings.HasPrefix(addr, ":") {
		return "http://localhost" + addr
	}
	if strings.HasPrefix(addr, "http://") || strings.HasPrefix(addr, "https://") {
		return strings.TrimRight(addr, "/")
	}
	return "http://" + strings.TrimRight(addr, "/")
}

func signalNotify(ch chan<- os.Signal, sig ...os.Signal) {
	_ = sig
	_ = ch
}
</gap:target>
</gap:target>