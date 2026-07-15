package gap

import (
	"strings"
	"testing"
)

func TestArtifactStoreApplyAndVersionConflict(t *testing.T) {
	store := NewArtifactStore(10)
	if _, _, err := store.Apply(synthEnv(t, "t", 1, "text/html", `<gap:target id="msg">hello</gap:target>`)); err != nil {
		t.Fatalf("synthesize: %v", err)
	}
	if version, ok := store.CurrentVersion("t"); !ok || version != 1 {
		t.Fatalf("version = %d %v, want 1 true", version, ok)
	}

	_, _, err := store.Apply(editEnv(t, "t", 6, "text/html", []EditOp{{
		Op:      OpTypeReplace,
		Target:  Target{Type: TargetTypeID, Value: "msg"},
		Content: stringPtr("world"),
	}}))
	if err == nil || !strings.Contains(err.Error(), "version conflict") {
		t.Fatalf("version conflict error = %v", err)
	}

	_, _, err = store.Apply(editEnv(t, "t", 0, "text/html", nil))
	if err == nil || !strings.Contains(err.Error(), "version must be >= 1") {
		t.Fatalf("version zero error = %v", err)
	}
}

func TestArtifactStoreRollbackAndChecksum(t *testing.T) {
	store := NewArtifactStore(10)
	if _, _, err := store.Apply(synthEnv(t, "t", 1, "text/html", "v1")); err != nil {
		t.Fatalf("v1: %v", err)
	}
	if _, _, err := store.Apply(synthEnv(t, "t", 2, "text/html", "v2")); err != nil {
		t.Fatalf("v2: %v", err)
	}
	rolled, err := store.Rollback("t", 1)
	if err != nil {
		t.Fatalf("rollback: %v", err)
	}
	if rolled.Body != "v1" || rolled.Version != 3 {
		t.Fatalf("rolled = %#v", rolled)
	}
	checksum, err := store.Checksum("t")
	if err != nil {
		t.Fatalf("checksum: %v", err)
	}
	if !strings.HasPrefix(checksum, "sha256:") {
		t.Fatalf("checksum = %q", checksum)
	}
}
