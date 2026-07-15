package gap

import "testing"

func TestMarkersFor(t *testing.T) {
	start, end, err := MarkersFor("nav", "text/html")
	if err != nil {
		t.Fatalf("MarkersFor returned error: %v", err)
	}
	if start != `<gap:target id="nav">` || end != `</gap:target>` {
		t.Fatalf("markers = %q %q", start, end)
	}
	if _, _, err := MarkersFor("data", "application/json"); err == nil {
		t.Fatal("expected JSON marker error")
	}
}

func TestFindTargetRangeNested(t *testing.T) {
	content := `<gap:target id="outer" type="section"><gap:target id="inner" type="value">val</gap:target></gap:target>`
	start, end, err := FindTargetRange(content, "inner", "text/html")
	if err != nil {
		t.Fatalf("find inner: %v", err)
	}
	if got := content[start:end]; got != "val" {
		t.Fatalf("inner content = %q", got)
	}
	start, end, err = FindTargetRange(content, "outer", "text/html")
	if err != nil {
		t.Fatalf("find outer: %v", err)
	}
	want := `<gap:target id="inner" type="value">val</gap:target>`
	if got := content[start:end]; got != want {
		t.Fatalf("outer content = %q, want %q", got, want)
	}
}

func TestFindTargetRangeInclusive(t *testing.T) {
	content := `before<gap:target id="x">data</gap:target>after`
	start, end, err := FindTargetRangeInclusive(content, "x", "text/html")
	if err != nil {
		t.Fatalf("FindTargetRangeInclusive: %v", err)
	}
	if got := content[start:end]; got != `<gap:target id="x">data</gap:target>` {
		t.Fatalf("inclusive range = %q", got)
	}
}

func TestExtractTargets(t *testing.T) {
	content := `<gap:target id="outer" type="section"><gap:target id="inner" type="value">v</gap:target></gap:target>`
	got := ExtractTargets(content, "text/html")
	want := []string{"outer", "inner"}
	if len(got) != len(want) {
		t.Fatalf("targets len = %d, want %d: %#v", len(got), len(want), got)
	}
	for i := range want {
		if got[i] != want[i] {
			t.Fatalf("targets[%d] = %q, want %q", i, got[i], want[i])
		}
	}
	if got := ExtractTargets(content, "application/json"); len(got) != 0 {
		t.Fatalf("JSON targets = %#v, want empty", got)
	}
}
