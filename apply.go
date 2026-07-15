package gap

import (
	"bytes"
	"encoding/json"
	"fmt"
	"strconv"
	"strings"
)

type TextResolver struct {
	Format string
}

func Apply(artifact *Artifact, envelope Envelope) (Artifact, Envelope, error) {
	format := envelope.Meta.ArtifactFormat()

	var result Artifact
	switch envelope.Name {
	case NameSynthesize:
		item, err := extractSynthesizeItem(envelope)
		if err != nil {
			return Artifact{}, Envelope{}, err
		}
		result = Artifact{
			ID:      envelope.ID,
			Version: envelope.Version,
			Format:  format,
			Body:    item.Body,
		}
	case NameEdit:
		if artifact == nil {
			return Artifact{}, Envelope{}, fmt.Errorf("edit requires a base artifact")
		}
		ops, err := extractEditOps(envelope)
		if err != nil {
			return Artifact{}, Envelope{}, err
		}
		body, err := applyEditBody(format, artifact.Body, ops)
		if err != nil {
			return Artifact{}, Envelope{}, err
		}
		result = Artifact{
			ID:      envelope.ID,
			Version: envelope.Version,
			Format:  format,
			Body:    body,
		}
	case NameHandle:
		return Artifact{}, Envelope{}, fmt.Errorf("handle is an output envelope, not an input operation")
	default:
		return Artifact{}, Envelope{}, fmt.Errorf("unsupported envelope name: %s", envelope.Name)
	}

	handle, err := buildHandleEnvelope(result)
	if err != nil {
		return Artifact{}, Envelope{}, err
	}
	return result, handle, nil
}

func ApplyEdit(resolver TextResolver, base string, operations []EditOp) (string, error) {
	content := base
	for i, op := range operations {
		start, end, err := resolveTarget(resolver, content, op.Target)
		if err != nil {
			return "", fmt.Errorf("operation %d: target not found: %w", i, err)
		}
		switch op.Op {
		case OpTypeReplace:
			content = content[:start] + optionalContent(op.Content) + content[end:]
		case OpTypeDelete:
			content = content[:start] + content[end:]
		case OpTypeInsertBefore:
			content = content[:start] + optionalContent(op.Content) + content[start:]
		case OpTypeInsertAfter:
			content = content[:end] + optionalContent(op.Content) + content[end:]
		default:
			return "", fmt.Errorf("operation %d: unsupported op: %s", i, op.Op)
		}
	}
	return content, nil
}

func extractSynthesizeItem(envelope Envelope) (SynthesizeContentItem, error) {
	if len(envelope.Content) == 0 {
		return SynthesizeContentItem{}, fmt.Errorf("synthesize: empty content array")
	}
	var item SynthesizeContentItem
	if err := json.Unmarshal(envelope.Content[0], &item); err != nil {
		return SynthesizeContentItem{}, fmt.Errorf("synthesize: failed to parse content item: %w", err)
	}
	return item, nil
}

func extractEditOps(envelope Envelope) ([]EditOp, error) {
	ops := make([]EditOp, 0, len(envelope.Content))
	for _, raw := range envelope.Content {
		var op EditOp
		if err := json.Unmarshal(raw, &op); err != nil {
			return nil, fmt.Errorf("edit: failed to parse content items: %w", err)
		}
		ops = append(ops, op)
	}
	return ops, nil
}

func applyEditBody(format, base string, operations []EditOp) (string, error) {
	hasPointer := false
	for _, op := range operations {
		if op.Target.Type == TargetTypePointer {
			hasPointer = true
			break
		}
	}
	if hasPointer {
		return applyEditPointers(base, operations)
	}
	return ApplyEdit(TextResolver{Format: format}, base, operations)
}

func buildHandleEnvelope(artifact Artifact) (Envelope, error) {
	targetIDs := ExtractTargets(artifact.Body, artifact.Format)
	targets := make([]TargetInfo, 0, len(targetIDs))
	for _, id := range targetIDs {
		targets = append(targets, TargetInfo{ID: id})
	}

	tokenCount := uint64(len(artifact.Body) / 4)
	item := HandleContentItem{
		ID:         artifact.ID,
		Version:    artifact.Version,
		TokenCount: &tokenCount,
		Targets:    targets,
	}
	raw, err := rawJSON(item)
	if err != nil {
		return Envelope{}, fmt.Errorf("failed to serialize handle: %w", err)
	}
	format := artifact.Format
	return Envelope{
		Protocol: ProtocolVersion,
		ID:       artifact.ID,
		Version:  artifact.Version,
		Name:     NameHandle,
		Meta:     Meta{Format: &format},
		Content:  []json.RawMessage{raw},
	}, nil
}

func resolveTarget(resolver TextResolver, content string, target Target) (int, int, error) {
	switch target.Type {
	case TargetTypeID:
		return FindTargetRange(content, target.Value, resolver.Format)
	case TargetTypePointer:
		value, err := decodeJSON(content)
		if err != nil {
			return 0, 0, fmt.Errorf("pointer targeting requires valid JSON content: %w", err)
		}
		if _, err := getPointer(value, target.Value); err != nil {
			return 0, 0, err
		}
		serialized, err := marshalJSONPretty(value)
		if err != nil {
			return 0, 0, err
		}
		return 0, len(serialized), nil
	default:
		return 0, 0, fmt.Errorf("unsupported target type: %s", target.Type)
	}
}

func applyEditPointers(base string, operations []EditOp) (string, error) {
	value, err := decodeJSON(base)
	if err != nil {
		return "", fmt.Errorf("pointer targeting requires valid JSON content: %w", err)
	}

	for i, op := range operations {
		if op.Target.Type != TargetTypePointer {
			return "", fmt.Errorf("operation %d: expected pointer target", i)
		}
		pointer := op.Target.Value
		switch op.Op {
		case OpTypeReplace:
			if op.Content == nil {
				return "", fmt.Errorf("replace requires content")
			}
			newValue, err := decodeJSON(*op.Content)
			if err != nil {
				return "", fmt.Errorf("content must be valid JSON: %w", err)
			}
			if err := setPointer(&value, pointer, newValue); err != nil {
				return "", err
			}
		case OpTypeDelete:
			parentPointer, key, ok := splitPointer(pointer)
			if !ok {
				return "", fmt.Errorf("cannot delete root")
			}
			if err := removeChild(&value, parentPointer, key); err != nil {
				return "", err
			}
		case OpTypeInsertBefore, OpTypeInsertAfter:
			if op.Content == nil {
				return "", fmt.Errorf("insert requires content")
			}
			newValue, err := decodeJSON(*op.Content)
			if err != nil {
				return "", fmt.Errorf("content must be valid JSON: %w", err)
			}
			parentPointer, key, ok := splitPointer(pointer)
			if !ok {
				return "", fmt.Errorf("cannot insert at root")
			}
			if err := insertChild(&value, parentPointer, key, op.Op, newValue); err != nil {
				return "", err
			}
		default:
			return "", fmt.Errorf("operation %d: unsupported op: %s", i, op.Op)
		}
	}

	return marshalJSONPretty(value)
}

func optionalContent(s *string) string {
	if s == nil {
		return ""
	}
	return *s
}

func decodeJSON(s string) (any, error) {
	decoder := json.NewDecoder(strings.NewReader(s))
	decoder.UseNumber()
	var value any
	if err := decoder.Decode(&value); err != nil {
		return nil, err
	}
	return value, nil
}

func marshalJSONPretty(value any) (string, error) {
	var buf bytes.Buffer
	encoder := json.NewEncoder(&buf)
	encoder.SetEscapeHTML(false)
	encoder.SetIndent("", "  ")
	if err := encoder.Encode(value); err != nil {
		return "", err
	}
	return strings.TrimSuffix(buf.String(), "\n"), nil
}

func parsePointer(pointer string) ([]string, error) {
	if pointer == "" {
		return nil, nil
	}
	if !strings.HasPrefix(pointer, "/") {
		return nil, fmt.Errorf("invalid JSON Pointer: %q", pointer)
	}
	rawParts := strings.Split(pointer[1:], "/")
	parts := make([]string, len(rawParts))
	for i, part := range rawParts {
		parts[i] = strings.ReplaceAll(strings.ReplaceAll(part, "~1", "/"), "~0", "~")
	}
	return parts, nil
}

func splitPointer(pointer string) (string, string, bool) {
	if pointer == "" || !strings.HasPrefix(pointer, "/") {
		return "", "", false
	}
	pos := strings.LastIndex(pointer, "/")
	if pos == 0 {
		key := pointer[1:]
		return "", strings.ReplaceAll(strings.ReplaceAll(key, "~1", "/"), "~0", "~"), true
	}
	parent := pointer[:pos]
	key := pointer[pos+1:]
	return parent, strings.ReplaceAll(strings.ReplaceAll(key, "~1", "/"), "~0", "~"), true
}

func getPointer(root any, pointer string) (any, error) {
	parts, err := parsePointer(pointer)
	if err != nil {
		return nil, err
	}
	return getAt(root, parts)
}

func getAt(root any, parts []string) (any, error) {
	current := root
	for _, part := range parts {
		switch node := current.(type) {
		case map[string]any:
			next, ok := node[part]
			if !ok {
				return nil, fmt.Errorf("pointer not found: %s", part)
			}
			current = next
		case []any:
			index, err := strconv.Atoi(part)
			if err != nil {
				return nil, fmt.Errorf("expected array index: %s", part)
			}
			if index < 0 || index >= len(node) {
				return nil, fmt.Errorf("array index out of bounds: %d", index)
			}
			current = node[index]
		default:
			return nil, fmt.Errorf("pointer parent is neither object nor array")
		}
	}
	return current, nil
}

func setPointer(root *any, pointer string, value any) error {
	parts, err := parsePointer(pointer)
	if err != nil {
		return err
	}
	return setAt(root, parts, value)
}

func setAt(root *any, parts []string, value any) error {
	if len(parts) == 0 {
		*root = value
		return nil
	}
	parent, err := getAt(*root, parts[:len(parts)-1])
	if err != nil {
		return err
	}
	key := parts[len(parts)-1]
	switch node := parent.(type) {
	case map[string]any:
		if _, ok := node[key]; !ok {
			return fmt.Errorf("pointer not found: %s", key)
		}
		node[key] = value
	case []any:
		index, err := strconv.Atoi(key)
		if err != nil {
			return fmt.Errorf("expected array index: %s", key)
		}
		if index < 0 || index >= len(node) {
			return fmt.Errorf("array index out of bounds: %d", index)
		}
		node[index] = value
	default:
		return fmt.Errorf("pointer parent is neither object nor array")
	}
	return nil
}

func removeChild(root *any, parentPointer, key string) error {
	parentParts, err := parsePointer(parentPointer)
	if err != nil {
		return err
	}
	parent, err := getAt(*root, parentParts)
	if err != nil {
		return err
	}
	switch node := parent.(type) {
	case map[string]any:
		if _, ok := node[key]; !ok {
			return fmt.Errorf("key not found: %s", key)
		}
		delete(node, key)
	case []any:
		index, err := strconv.Atoi(key)
		if err != nil {
			return fmt.Errorf("expected array index: %s", key)
		}
		if index < 0 || index >= len(node) {
			return fmt.Errorf("array index out of bounds: %d", index)
		}
		next := append(append([]any{}, node[:index]...), node[index+1:]...)
		return setAt(root, parentParts, next)
	default:
		return fmt.Errorf("parent is neither object nor array")
	}
	return nil
}

func insertChild(root *any, parentPointer, key string, op OpType, value any) error {
	parentParts, err := parsePointer(parentPointer)
	if err != nil {
		return err
	}
	parent, err := getAt(*root, parentParts)
	if err != nil {
		return err
	}
	arr, ok := parent.([]any)
	if !ok {
		return fmt.Errorf("insert requires array parent")
	}
	index, err := strconv.Atoi(key)
	if err != nil {
		return fmt.Errorf("insert requires numeric array index")
	}
	insertAt := index
	if op == OpTypeInsertAfter {
		insertAt = index + 1
	}
	if insertAt < 0 || insertAt > len(arr) {
		return fmt.Errorf("insert index %d out of bounds for array of len %d", insertAt, len(arr))
	}
	next := append(arr[:insertAt:insertAt], append([]any{value}, arr[insertAt:]...)...)
	return setAt(root, parentParts, next)
}
