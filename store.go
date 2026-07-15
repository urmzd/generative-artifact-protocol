package gap

import (
	"crypto/sha256"
	"fmt"
)

type ArtifactStore struct {
	history    map[string][]Artifact
	maxHistory int
}

func NewArtifactStore(maxHistory int) *ArtifactStore {
	return &ArtifactStore{
		history:    make(map[string][]Artifact),
		maxHistory: maxHistory,
	}
}

func (s *ArtifactStore) Get(id string) (Artifact, bool) {
	entries := s.history[id]
	if len(entries) == 0 {
		return Artifact{}, false
	}
	return entries[len(entries)-1], true
}

func (s *ArtifactStore) CurrentVersion(id string) (uint64, bool) {
	artifact, ok := s.Get(id)
	if !ok {
		return 0, false
	}
	return artifact.Version, true
}

func (s *ArtifactStore) Apply(envelope Envelope) (Artifact, Envelope, error) {
	if envelope.Name != NameSynthesize {
		if envelope.Version == 0 {
			return Artifact{}, Envelope{}, fmt.Errorf("invalid envelope version 0: version must be >= 1")
		}
		expectedPrevious := envelope.Version - 1
		current, ok := s.CurrentVersion(envelope.ID)
		if !ok {
			return Artifact{}, Envelope{}, fmt.Errorf("no base artifact for %q — synthesize first", envelope.ID)
		}
		if current != expectedPrevious {
			return Artifact{}, Envelope{}, fmt.Errorf("version conflict: stored=%d, envelope=%d, expected=%d", current, envelope.Version, expectedPrevious)
		}
	}

	var base *Artifact
	if artifact, ok := s.Get(envelope.ID); ok {
		base = &artifact
	}
	newArtifact, handle, err := Apply(base, envelope)
	if err != nil {
		return Artifact{}, Envelope{}, err
	}

	entries := append(s.history[envelope.ID], newArtifact)
	for len(entries) > s.maxHistory {
		entries = entries[1:]
	}
	s.history[envelope.ID] = entries
	return newArtifact, handle, nil
}

func (s *ArtifactStore) Checksum(id string) (string, error) {
	artifact, ok := s.Get(id)
	if !ok {
		return "", fmt.Errorf("artifact not found")
	}
	sum := sha256.Sum256([]byte(artifact.Body))
	return fmt.Sprintf("sha256:%x", sum), nil
}

func (s *ArtifactStore) Rollback(id string, targetVersion uint64) (Artifact, error) {
	entries := s.history[id]
	if len(entries) == 0 {
		return Artifact{}, fmt.Errorf("artifact not found")
	}
	index := -1
	for i, artifact := range entries {
		if artifact.Version == targetVersion {
			index = i
			break
		}
	}
	if index < 0 {
		return Artifact{}, fmt.Errorf("version %d not in history", targetVersion)
	}
	rolled := entries[index]
	rolled.Version = entries[len(entries)-1].Version + 1
	entries = append(entries, rolled)
	for len(entries) > s.maxHistory {
		entries = entries[1:]
	}
	s.history[id] = entries
	return rolled, nil
}
