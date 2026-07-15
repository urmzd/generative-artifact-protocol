package liveeval

import (
	"encoding/json"
	"fmt"
	"regexp"
	"sort"
	"strconv"
	"strings"

	gap "github.com/urmzd/generative-artifact-protocol"
)

const maxInventoryEntries = 180

type inventoryEntry struct {
	Key     string
	Snippet string
	Score   int
}

func validateSynthesisArtifact(artifact string, format string) error {
	if format == "application/json" {
		var value any
		if err := json.Unmarshal([]byte(artifact), &value); err != nil {
			return fmt.Errorf("validate_json_artifact failed: %w", err)
		}
		return nil
	}
	if len(gap.ExtractTargets(artifact, format)) == 0 {
		return fmt.Errorf("validate_anchors failed: no GAP target markers found")
	}
	return nil
}

func synthesisRepairPrompt(artifact string, format string, reason error) string {
	return fmt.Sprintf(`## Supervisor Tool Error

%s

The artifact must be regenerated before edit turns can run.

For non-JSON formats, wrap stable editable slots with <gap:target id="...">...</gap:target>.
Use deterministic IDs that future edits can address. For records, include IDs for the whole record and common fields, for example record-087 and record-087-price.

For JSON, return valid raw JSON with no GAP marker tags.

## Rejected Artifact Preview

`+"```"+`
%s
`+"```"+`

Return the corrected raw %s artifact only. No markdown fences and no explanation.`, reason.Error(), truncate(artifact, 6000), format)
}

func editPrompt(artifact string, format string, instruction string) string {
	return fmt.Sprintf(`## Current Artifact

`+"```"+`
%s
`+"```"+`

## Edit Instruction

%s

%s

## Required Output

Return exactly one JSON GAP edit envelope with `+"`name`"+` set to `+"`edit`"+` and `+"`content`"+` set to an array of edit operations. Each operation must have `+"`op`"+`, `+"`target`"+`, and `+"`content`"+`.

Before choosing a target, use the supervisor inventory above:
- For non-JSON artifacts, target only IDs returned by list_targets().
- For JSON artifacts, target only JSON Pointers returned by list_paths().
- Do not invent target IDs or paths.
- Put replacement text in the operation `+"`content`"+` field.`, artifact, instruction, supervisorInventory(artifact, format, instruction))
}

func repairPrompt(artifact string, format string, instruction string, validationErr error) string {
	return fmt.Sprintf(`## Supervisor Tool Error

validate_edit_envelope failed:

%s

The previous envelope was rejected before apply. Produce a corrected GAP edit envelope.

%s`, validationErr.Error(), editPrompt(artifact, format, instruction))
}

func validateEnvelope(artifact string, format string, envelope gap.Envelope) error {
	if envelope.Name != gap.NameEdit {
		return fmt.Errorf("invalid envelope name %q; expected %q", envelope.Name, gap.NameEdit)
	}
	if len(envelope.Content) == 0 {
		return fmt.Errorf("invalid envelope: content must contain at least one edit operation")
	}
	ops, err := envelopeOps(envelope)
	if err != nil {
		return err
	}
	if format == "application/json" {
		for i, op := range ops {
			if op.Target.Type != gap.TargetTypePointer {
				return fmt.Errorf("operation %d: JSON artifacts require target.type=%q; got %q", i, gap.TargetTypePointer, op.Target.Type)
			}
			if op.Op != gap.OpTypeDelete && op.Content == nil {
				return fmt.Errorf("operation %d: %s requires content", i, op.Op)
			}
			if op.Content != nil && !json.Valid([]byte(*op.Content)) {
				return fmt.Errorf("operation %d: content must be a serialized JSON value", i)
			}
		}
		return nil
	}

	targets := targetSet(artifact, format)
	if len(targets) == 0 {
		return fmt.Errorf("list_targets returned no target IDs; regenerate the artifact with GAP markers before editing")
	}
	for i, op := range ops {
		if op.Target.Type != gap.TargetTypeID {
			return fmt.Errorf("operation %d: %s artifacts require target.type=%q; got %q", i, format, gap.TargetTypeID, op.Target.Type)
		}
		if _, ok := targets[op.Target.Value]; !ok {
			return fmt.Errorf("operation %d: validate_target(%q) failed: target ID does not exist", i, op.Target.Value)
		}
		if _, _, err := gap.FindTargetRange(artifact, op.Target.Value, format); err != nil {
			return fmt.Errorf("operation %d: validate_target(%q) failed: %w", i, op.Target.Value, err)
		}
	}
	return nil
}

func envelopeOps(envelope gap.Envelope) ([]gap.EditOp, error) {
	ops := make([]gap.EditOp, 0, len(envelope.Content))
	for i, raw := range envelope.Content {
		var op gap.EditOp
		if err := json.Unmarshal(raw, &op); err != nil {
			return nil, fmt.Errorf("operation %d: invalid edit operation: %w", i, err)
		}
		ops = append(ops, op)
	}
	return ops, nil
}

func supervisorInventory(artifact string, format string, instruction string) string {
	if format == "application/json" {
		entries, err := jsonPathInventory(artifact, instruction)
		if err != nil {
			return "## Supervisor Target Tools\n\nlist_paths() failed: " + err.Error()
		}
		return "## Supervisor Target Tools\n\nlist_paths() returned these valid JSON Pointers:\n\n" + formatInventory(entries)
	}
	entries := targetInventory(artifact, format, instruction)
	if len(entries) == 0 {
		return "## Supervisor Target Tools\n\nlist_targets() returned no target IDs. The artifact must be regenerated with GAP markers before an ID-targeted edit can apply."
	}
	return "## Supervisor Target Tools\n\nlist_targets() returned these valid target IDs:\n\n" + formatInventory(entries)
}

func targetInventory(artifact string, format string, instruction string) []inventoryEntry {
	seen := map[string]bool{}
	var entries []inventoryEntry
	for _, id := range gap.ExtractTargets(artifact, format) {
		if seen[id] {
			continue
		}
		seen[id] = true
		start, end, err := gap.FindTargetRange(artifact, id, format)
		snippet := ""
		if err == nil {
			snippet = oneLine(artifact[start:end])
		}
		entries = append(entries, inventoryEntry{
			Key:     id,
			Snippet: truncate(snippet, 160),
			Score:   scoreInventory(id+" "+snippet, instruction),
		})
	}
	sortInventory(entries)
	return limitInventory(entries)
}

func targetSet(artifact string, format string) map[string]struct{} {
	targets := map[string]struct{}{}
	for _, id := range gap.ExtractTargets(artifact, format) {
		targets[id] = struct{}{}
	}
	return targets
}

func jsonPathInventory(artifact string, instruction string) ([]inventoryEntry, error) {
	var value any
	if err := json.Unmarshal([]byte(artifact), &value); err != nil {
		return nil, err
	}
	var entries []inventoryEntry
	walkJSONPaths(&entries, "", value, instruction)
	sortInventory(entries)
	return limitInventory(entries), nil
}

func walkJSONPaths(entries *[]inventoryEntry, path string, value any, instruction string) {
	snippet := jsonSnippet(value)
	if path != "" {
		*entries = append(*entries, inventoryEntry{
			Key:     path,
			Snippet: truncate(snippet, 160),
			Score:   scoreInventory(path+" "+snippet, instruction),
		})
	}
	switch node := value.(type) {
	case map[string]any:
		keys := make([]string, 0, len(node))
		for key := range node {
			keys = append(keys, key)
		}
		sort.Strings(keys)
		for _, key := range keys {
			walkJSONPaths(entries, path+"/"+escapePointerPart(key), node[key], instruction)
		}
	case []any:
		for i, item := range node {
			walkJSONPaths(entries, path+"/"+strconv.Itoa(i), item, instruction)
		}
	}
}

func jsonSnippet(value any) string {
	data, err := json.Marshal(value)
	if err != nil {
		return ""
	}
	return oneLine(string(data))
}

func escapePointerPart(part string) string {
	part = strings.ReplaceAll(part, "~", "~0")
	return strings.ReplaceAll(part, "/", "~1")
}

func formatInventory(entries []inventoryEntry) string {
	if len(entries) == 0 {
		return "(none)"
	}
	var b strings.Builder
	for _, entry := range entries {
		if entry.Snippet == "" {
			fmt.Fprintf(&b, "- %s\n", entry.Key)
			continue
		}
		fmt.Fprintf(&b, "- %s: %s\n", entry.Key, entry.Snippet)
	}
	return strings.TrimRight(b.String(), "\n")
}

func sortInventory(entries []inventoryEntry) {
	sort.SliceStable(entries, func(i, j int) bool {
		if entries[i].Score != entries[j].Score {
			return entries[i].Score > entries[j].Score
		}
		return entries[i].Key < entries[j].Key
	})
}

func limitInventory(entries []inventoryEntry) []inventoryEntry {
	if len(entries) <= maxInventoryEntries {
		return entries
	}
	return entries[:maxInventoryEntries]
}

var inventoryTermRE = regexp.MustCompile(`[a-zA-Z0-9_.:/{}-]+`)

func scoreInventory(text string, instruction string) int {
	text = strings.ToLower(text)
	score := 0
	for _, term := range inventoryTermRE.FindAllString(strings.ToLower(instruction), -1) {
		if len(term) < 2 {
			continue
		}
		if strings.Contains(text, term) {
			score += len(term)
		}
	}
	return score
}

func oneLine(value string) string {
	return strings.Join(strings.Fields(value), " ")
}
