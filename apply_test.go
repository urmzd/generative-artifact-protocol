package gap

import (
	"encoding/json"
	"strings"
	"testing"
)

func TestApplySynthesizeThenEdit(t *testing.T) {
	env := synthEnv(t, "greeting", 1, "text/html", `<gap:target id="msg">hello</gap:target>`)
	artifact, handle, err := Apply(nil, env)
	if err != nil {
		t.Fatalf("synthesize: %v", err)
	}
	if !strings.Contains(artifact.Body, "hello") {
		t.Fatalf("artifact body = %q", artifact.Body)
	}
	if handle.Name != NameHandle {
		t.Fatalf("handle name = %q", handle.Name)
	}

	edit := editEnv(t, "greeting", 2, "text/html", []EditOp{{
		Op:      OpTypeReplace,
		Target:  Target{Type: TargetTypeID, Value: "msg"},
		Content: stringPtr("world"),
	}})
	updated, _, err := Apply(&artifact, edit)
	if err != nil {
		t.Fatalf("edit: %v", err)
	}
	if !strings.Contains(updated.Body, "world") {
		t.Fatalf("updated body = %q", updated.Body)
	}
	if !strings.Contains(updated.Body, `<gap:target id="msg">`) {
		t.Fatalf("edit should preserve markers: %q", updated.Body)
	}
}

func TestApplyEditOperations(t *testing.T) {
	base := `<gap:target id="x">middle</gap:target>`
	tests := []struct {
		name string
		op   EditOp
		want string
	}{
		{
			name: "replace",
			op:   EditOp{Op: OpTypeReplace, Target: Target{Type: TargetTypeID, Value: "x"}, Content: stringPtr("new")},
			want: `<gap:target id="x">new</gap:target>`,
		},
		{
			name: "delete",
			op:   EditOp{Op: OpTypeDelete, Target: Target{Type: TargetTypeID, Value: "x"}},
			want: `<gap:target id="x"></gap:target>`,
		},
		{
			name: "insert before",
			op:   EditOp{Op: OpTypeInsertBefore, Target: Target{Type: TargetTypeID, Value: "x"}, Content: stringPtr("pre-")},
			want: `<gap:target id="x">pre-middle</gap:target>`,
		},
		{
			name: "insert after",
			op:   EditOp{Op: OpTypeInsertAfter, Target: Target{Type: TargetTypeID, Value: "x"}, Content: stringPtr("-post")},
			want: `<gap:target id="x">middle-post</gap:target>`,
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := ApplyEdit(TextResolver{Format: "text/html"}, base, []EditOp{tt.op})
			if err != nil {
				t.Fatalf("ApplyEdit: %v", err)
			}
			if got != tt.want {
				t.Fatalf("body = %q, want %q", got, tt.want)
			}
		})
	}
}

func TestApplyPointerOperations(t *testing.T) {
	base := mustArtifact(`{"items":[1,2,3],"user":{"name":"Ada","active":true}}`)
	ops := []EditOp{
		{
			Op:      OpTypeReplace,
			Target:  Target{Type: TargetTypePointer, Value: "/user/name"},
			Content: stringPtr(`"Grace"`),
		},
		{
			Op:      OpTypeInsertAfter,
			Target:  Target{Type: TargetTypePointer, Value: "/items/1"},
			Content: stringPtr(`99`),
		},
		{
			Op:     OpTypeDelete,
			Target: Target{Type: TargetTypePointer, Value: "/user/active"},
		},
	}
	env := editEnv(t, "json-doc", 2, "application/json", ops)
	updated, _, err := Apply(&base, env)
	if err != nil {
		t.Fatalf("Apply pointer edit: %v", err)
	}
	var got map[string]any
	if err := json.Unmarshal([]byte(updated.Body), &got); err != nil {
		t.Fatalf("updated JSON invalid: %v\n%s", err, updated.Body)
	}
	if got["user"].(map[string]any)["name"] != "Grace" {
		t.Fatalf("name not updated: %#v", got)
	}
	if _, ok := got["user"].(map[string]any)["active"]; ok {
		t.Fatalf("active should be deleted: %#v", got)
	}
	items := got["items"].([]any)
	if len(items) != 4 || items[2].(float64) != 99 {
		t.Fatalf("items = %#v", items)
	}
}

func TestApplyPointerErrors(t *testing.T) {
	base := mustArtifact(`{"items":[1,2,3]}`)
	tests := []struct {
		name string
		ops  []EditOp
		want string
	}{
		{
			name: "replace missing content",
			ops:  []EditOp{{Op: OpTypeReplace, Target: Target{Type: TargetTypePointer, Value: "/items/1"}}},
			want: "replace requires content",
		},
		{
			name: "insert out of bounds",
			ops:  []EditOp{{Op: OpTypeInsertAfter, Target: Target{Type: TargetTypePointer, Value: "/items/99"}, Content: stringPtr("4")}},
			want: "out of bounds",
		},
		{
			name: "mixed target modes",
			ops: []EditOp{
				{Op: OpTypeReplace, Target: Target{Type: TargetTypePointer, Value: "/items/0"}, Content: stringPtr("4")},
				{Op: OpTypeReplace, Target: Target{Type: TargetTypeID, Value: "x"}, Content: stringPtr("4")},
			},
			want: "expected pointer target",
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			_, _, err := Apply(&base, editEnv(t, "json-doc", 2, "application/json", tt.ops))
			if err == nil || !strings.Contains(err.Error(), tt.want) {
				t.Fatalf("error = %v, want containing %q", err, tt.want)
			}
		})
	}
}

func TestApplyRejectsInvalidInputs(t *testing.T) {
	if _, _, err := Apply(nil, editEnv(t, "x", 1, "text/html", nil)); err == nil {
		t.Fatal("expected edit without base to fail")
	}
	if _, _, err := Apply(nil, Envelope{Name: NameHandle}); err == nil {
		t.Fatal("expected handle input to fail")
	}
	if _, _, err := Apply(nil, Envelope{Name: NameSynthesize}); err == nil || !strings.Contains(err.Error(), "empty content") {
		t.Fatalf("synthesize error = %v", err)
	}
}

func TestHandleIncludesTargetsForTextButNotJSON(t *testing.T) {
	artifact, handle, err := Apply(nil, synthEnv(t, "x", 1, "text/html", `<gap:target id="a">one</gap:target>`))
	if err != nil {
		t.Fatalf("synthesize: %v", err)
	}
	var item HandleContentItem
	if err := json.Unmarshal(handle.Content[0], &item); err != nil {
		t.Fatalf("handle content: %v", err)
	}
	if item.ID != artifact.ID || len(item.Targets) != 1 || item.Targets[0].ID != "a" {
		t.Fatalf("handle item = %#v", item)
	}

	jsonArtifact := mustArtifact(`{"x":1}`)
	jsonArtifact.Format = "application/json"
	jsonHandle, err := buildHandleEnvelope(jsonArtifact)
	if err != nil {
		t.Fatalf("build JSON handle: %v", err)
	}
	item = HandleContentItem{}
	if err := json.Unmarshal(jsonHandle.Content[0], &item); err != nil {
		t.Fatalf("json handle content: %v", err)
	}
	if len(item.Targets) != 0 {
		t.Fatalf("JSON handle targets = %#v, want empty", item.Targets)
	}
}

func synthEnv(t *testing.T, id string, version uint64, format, body string) Envelope {
	t.Helper()
	raw, err := rawJSON(SynthesizeContentItem{Body: body})
	if err != nil {
		t.Fatalf("marshal synth content: %v", err)
	}
	return Envelope{
		Protocol: ProtocolVersion,
		ID:       id,
		Version:  version,
		Name:     NameSynthesize,
		Meta:     Meta{Format: &format},
		Content:  []json.RawMessage{raw},
	}
}

func editEnv(t *testing.T, id string, version uint64, format string, ops []EditOp) Envelope {
	t.Helper()
	content := make([]json.RawMessage, 0, len(ops))
	for _, op := range ops {
		raw, err := rawJSON(op)
		if err != nil {
			t.Fatalf("marshal edit op: %v", err)
		}
		content = append(content, raw)
	}
	return Envelope{
		Protocol: ProtocolVersion,
		ID:       id,
		Version:  version,
		Name:     NameEdit,
		Meta:     Meta{Format: &format},
		Content:  content,
	}
}

func mustArtifact(body string) Artifact {
	return Artifact{
		ID:      "json-doc",
		Version: 1,
		Format:  "application/json",
		Body:    body,
	}
}

func stringPtr(s string) *string {
	return &s
}
