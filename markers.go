package gap

import (
	"fmt"
	"regexp"
	"strings"
)

const (
	openPrefix = "<gap:target "
	closeTag   = "</gap:target>"
)

var targetOpenRE = regexp.MustCompile(`<gap:target\b[^>]*\sid="([^"]+)"[^>]*>`)

func MarkersFor(targetID, format string) (string, string, error) {
	if format == "application/json" {
		return "", "", fmt.Errorf("JSON does not support text-based markers; use pointer addressing instead")
	}
	return fmt.Sprintf(`<gap:target id="%s">`, targetID), closeTag, nil
}

func FindTargetRange(content, targetID, format string) (int, int, error) {
	_, _, err := MarkersFor(targetID, format)
	if err != nil {
		return 0, 0, err
	}
	startIndex, contentStart := findOpenMarker(content, targetID)
	if startIndex < 0 {
		return 0, 0, fmt.Errorf("start marker not found for target: %s", targetID)
	}
	endIndex, ok := findMatchingClose(content, contentStart)
	if !ok {
		return 0, 0, fmt.Errorf("end marker not found for target: %s", targetID)
	}
	return contentStart, endIndex, nil
}

func FindTargetRangeInclusive(content, targetID, format string) (int, int, error) {
	_, _, err := MarkersFor(targetID, format)
	if err != nil {
		return 0, 0, err
	}
	startIndex, contentStart := findOpenMarker(content, targetID)
	if startIndex < 0 {
		return 0, 0, fmt.Errorf("start marker not found for target: %s", targetID)
	}
	endIndex, ok := findMatchingClose(content, contentStart)
	if !ok {
		return 0, 0, fmt.Errorf("end marker not found for target: %s", targetID)
	}
	return startIndex, endIndex + len(closeTag), nil
}

func ExtractTargets(content, format string) []string {
	if format == "application/json" {
		return nil
	}
	matches := targetOpenRE.FindAllStringSubmatch(content, -1)
	targets := make([]string, 0, len(matches))
	for _, match := range matches {
		targets = append(targets, match[1])
	}
	return targets
}

func findOpenMarker(content, targetID string) (int, int) {
	re := regexp.MustCompile(`<gap:target\b[^>]*\sid="` + regexp.QuoteMeta(targetID) + `"[^>]*>`)
	loc := re.FindStringIndex(content)
	if loc == nil {
		return -1, -1
	}
	return loc[0], loc[1]
}

func findMatchingClose(content string, contentStart int) (int, bool) {
	depth := 1
	cursor := contentStart
	for cursor < len(content) && depth > 0 {
		nextOpen := indexFrom(content, openPrefix, cursor)
		nextClose := indexFrom(content, closeTag, cursor)
		switch {
		case nextOpen >= 0 && nextClose >= 0 && nextOpen < nextClose:
			depth++
			cursor = nextOpen + len(openPrefix)
		case nextClose >= 0:
			depth--
			if depth == 0 {
				return nextClose, true
			}
			cursor = nextClose + len(closeTag)
		default:
			return 0, false
		}
	}
	return 0, false
}

func indexFrom(s, needle string, from int) int {
	index := strings.Index(s[from:], needle)
	if index < 0 {
		return -1
	}
	return from + index
}
