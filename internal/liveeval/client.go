package liveeval

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"
)

type Message struct {
	Role    string `json:"role"`
	Content string `json:"content"`
}

type Client struct {
	HTTPClient  *http.Client
	APIBase     string
	APIKey      string
	Model       string
	Temperature *float64
	Seed        *int64
}

type ChatResult struct {
	Text              string
	InputTokens       uint64
	OutputTokens      uint64
	CachedInputTokens uint64
	Retried           bool
}

func NewClient(apiBase, apiKey, model string) *Client {
	apiBase = strings.TrimRight(apiBase, "/")
	if apiBase == "" {
		apiBase = "https://api.openai.com/v1"
	}
	if model == "" {
		model = "gpt-4o-mini"
	}
	var temperature *float64
	lower := strings.ToLower(model)
	if !strings.HasPrefix(lower, "o1") &&
		!strings.HasPrefix(lower, "o3") &&
		!strings.HasPrefix(lower, "o4") &&
		!strings.HasPrefix(lower, "gpt-5") {
		t := 0.0
		temperature = &t
	}
	seed := int64(42)
	return &Client{
		HTTPClient:  &http.Client{Timeout: 10 * time.Minute},
		APIBase:     apiBase,
		APIKey:      apiKey,
		Model:       model,
		Temperature: temperature,
		Seed:        &seed,
	}
}

func (c *Client) Chat(ctx context.Context, messages []Message, jsonMode bool) (ChatResult, error) {
	if c.APIKey == "" {
		return ChatResult{}, fmt.Errorf("missing API key")
	}

	body := map[string]any{
		"model":    c.Model,
		"messages": messages,
	}
	if c.Temperature != nil {
		body["temperature"] = *c.Temperature
	}
	if c.Seed != nil {
		body["seed"] = *c.Seed
	}
	if jsonMode {
		body["response_format"] = map[string]any{
			"type": "json_schema",
			"json_schema": map[string]any{
				"name":   "gap_envelope",
				"strict": true,
				"schema": envelopeSchema(),
			},
		}
	}

	data, err := json.Marshal(body)
	if err != nil {
		return ChatResult{}, err
	}

	var retried bool
	for attempt := 1; attempt <= 6; attempt++ {
		result, retry, err := c.doChat(ctx, data)
		if err == nil && !retry {
			result.Retried = retried
			return result, nil
		}
		if attempt == 6 || !retry {
			if err != nil {
				return ChatResult{}, err
			}
			return result, nil
		}
		retried = true
		select {
		case <-ctx.Done():
			return ChatResult{}, ctx.Err()
		case <-time.After(time.Duration(1<<min(attempt-1, 4)) * time.Second):
		}
	}
	return ChatResult{}, fmt.Errorf("unreachable retry state")
}

func envelopeSchema() map[string]any {
	return map[string]any{
		"type":                 "object",
		"additionalProperties": false,
		"required":             []string{"protocol", "id", "version", "name", "meta", "content"},
		"properties": map[string]any{
			"protocol": map[string]any{"type": "string"},
			"id":       map[string]any{"type": "string"},
			"version":  map[string]any{"type": "integer"},
			"name":     map[string]any{"type": "string", "enum": []string{"edit"}},
			"meta": map[string]any{
				"type":                 "object",
				"additionalProperties": false,
				"required":             []string{"format", "tokens_used", "checksum", "state"},
				"properties": map[string]any{
					"format":      map[string]any{"type": []string{"string", "null"}},
					"tokens_used": map[string]any{"type": []string{"integer", "null"}},
					"checksum":    map[string]any{"type": []string{"string", "null"}},
					"state":       map[string]any{"type": []string{"string", "null"}},
				},
			},
			"content": map[string]any{
				"type": "array",
				"items": map[string]any{
					"type":                 "object",
					"additionalProperties": false,
					"required":             []string{"op", "target", "content"},
					"properties": map[string]any{
						"op": map[string]any{
							"type": "string",
							"enum": []string{"replace", "insert_before", "insert_after", "delete"},
						},
						"target": map[string]any{
							"type":                 "object",
							"additionalProperties": false,
							"required":             []string{"type", "value"},
							"properties": map[string]any{
								"type":  map[string]any{"type": "string", "enum": []string{"id", "pointer"}},
								"value": map[string]any{"type": "string"},
							},
						},
						"content": map[string]any{"type": []string{"string", "null"}},
					},
				},
			},
		},
	}
}

func (c *Client) doChat(ctx context.Context, data []byte) (ChatResult, bool, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodPost, c.APIBase+"/chat/completions", bytes.NewReader(data))
	if err != nil {
		return ChatResult{}, false, err
	}
	req.Header.Set("Authorization", "Bearer "+c.APIKey)
	req.Header.Set("Content-Type", "application/json")

	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return ChatResult{}, true, err
	}
	defer resp.Body.Close()

	respData, err := io.ReadAll(resp.Body)
	if err != nil {
		return ChatResult{}, false, err
	}
	if resp.StatusCode == http.StatusTooManyRequests || resp.StatusCode >= 500 {
		return ChatResult{}, true, fmt.Errorf("API error %d: %s", resp.StatusCode, string(respData))
	}
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return ChatResult{}, false, fmt.Errorf("API error %d: %s", resp.StatusCode, string(respData))
	}

	var decoded chatResponse
	if err := json.Unmarshal(respData, &decoded); err != nil {
		return ChatResult{}, false, err
	}
	if len(decoded.Choices) == 0 {
		return ChatResult{}, false, fmt.Errorf("chat completion returned no choices")
	}

	return ChatResult{
		Text:              decoded.Choices[0].Message.Content,
		InputTokens:       decoded.Usage.PromptTokens,
		OutputTokens:      decoded.Usage.CompletionTokens,
		CachedInputTokens: decoded.Usage.PromptTokensDetails.CachedTokens,
	}, false, nil
}

type chatResponse struct {
	Choices []struct {
		Message struct {
			Content string `json:"content"`
		} `json:"message"`
	} `json:"choices"`
	Usage struct {
		PromptTokens        uint64 `json:"prompt_tokens"`
		CompletionTokens    uint64 `json:"completion_tokens"`
		PromptTokensDetails struct {
			CachedTokens uint64 `json:"cached_tokens"`
		} `json:"prompt_tokens_details"`
	} `json:"usage"`
}
