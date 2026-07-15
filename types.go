package gap

import "encoding/json"

const ProtocolVersion = "gap/0.1"

type Name string

const (
	NameSynthesize Name = "synthesize"
	NameEdit       Name = "edit"
	NameHandle     Name = "handle"
)

type ArtifactState string

const (
	ArtifactStateDraft     ArtifactState = "draft"
	ArtifactStatePublished ArtifactState = "published"
	ArtifactStateArchived  ArtifactState = "archived"
)

type Meta struct {
	Format     *string        `json:"format,omitempty"`
	TokensUsed *uint64        `json:"tokens_used,omitempty"`
	Checksum   *string        `json:"checksum,omitempty"`
	State      *ArtifactState `json:"state,omitempty"`
}

func (m Meta) ArtifactFormat() string {
	if m.Format == nil || *m.Format == "" {
		return "text/html"
	}
	return *m.Format
}

type Envelope struct {
	Protocol string            `json:"protocol"`
	ID       string            `json:"id"`
	Version  uint64            `json:"version"`
	Name     Name              `json:"name"`
	Meta     Meta              `json:"meta"`
	Content  []json.RawMessage `json:"content"`
}

func EnvelopeFromJSON(data []byte) (Envelope, error) {
	var envelope Envelope
	err := json.Unmarshal(data, &envelope)
	return envelope, err
}

type Artifact struct {
	ID      string `json:"id"`
	Version uint64 `json:"version"`
	Format  string `json:"format"`
	Body    string `json:"body"`
}

type SynthesizeContentItem struct {
	Body string `json:"body"`
}

type TargetType string

const (
	TargetTypeID      TargetType = "id"
	TargetTypePointer TargetType = "pointer"
)

type Target struct {
	Type  TargetType `json:"type"`
	Value string     `json:"value"`
}

type OpType string

const (
	OpTypeReplace      OpType = "replace"
	OpTypeInsertBefore OpType = "insert_before"
	OpTypeInsertAfter  OpType = "insert_after"
	OpTypeDelete       OpType = "delete"
)

type EditOp struct {
	Op      OpType  `json:"op"`
	Target  Target  `json:"target"`
	Content *string `json:"content,omitempty"`
}

type TargetInfo struct {
	ID      string  `json:"id"`
	Label   *string `json:"label,omitempty"`
	Accepts *string `json:"accepts,omitempty"`
}

type HandleContentItem struct {
	ID         string         `json:"id"`
	Version    uint64         `json:"version"`
	TokenCount *uint64        `json:"token_count,omitempty"`
	State      *ArtifactState `json:"state,omitempty"`
	Content    *string        `json:"content,omitempty"`
	Targets    []TargetInfo   `json:"targets,omitempty"`
}

func rawJSON(v any) (json.RawMessage, error) {
	data, err := json.Marshal(v)
	if err != nil {
		return nil, err
	}
	return json.RawMessage(data), nil
}
