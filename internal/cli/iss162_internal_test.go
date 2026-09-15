package cli

import (
	"os"
	"path/filepath"
	"runtime"
	"testing"
)

// TestStatusDiff_ModeDriftDetection exercises the two helpers that give status
// and diff their mode-drift awareness (issue #162 item D): modeDrifted upgrades a
// content-clean file whose recorded mode diverged from disk to `drift` in
// buildStatusModel, and modeHunk makes diff emit a "mode" hunk for a
// content-identical chmod that would otherwise read as "no diff". A file whose
// mode still matches, or whose recorded/intended mode is unspecified (0), stays a
// no-op — preserving the mtime-churn-avoidance intent.
func TestStatusDiff_ModeDriftDetection(t *testing.T) {
	dir := t.TempDir()
	p := filepath.Join(dir, "run.sh")
	if err := os.WriteFile(p, []byte("#!/bin/sh\n"), 0o755); err != nil {
		t.Fatal(err)
	}

	if runtime.GOOS == "windows" {
		// Windows exposes only the read-only attribute through Go's permission
		// bits, so a writable file may report 0666 for any requested mode that
		// includes a write bit. The helpers compare that effective capability.
		if modeDrifted(0o644, p) {
			t.Errorf("modeDrifted: writable Windows file must not drift from 0644")
		}
		if err := os.Chmod(p, 0o444); err != nil {
			t.Fatal(err)
		}
		if !modeDrifted(0o644, p) {
			t.Errorf("modeDrifted: read-only Windows file must drift from writable 0644")
		}
		src, dst, ok := modeHunk(p, 0o644)
		if !ok || src != "mode 0644" || dst != "mode 0444" {
			t.Errorf("modeHunk on Windows = %q / %q / %v, want 0644 / 0444 / true", src, dst, ok)
		}
		if _, _, ok := modeHunk(p, 0o444); ok {
			t.Errorf("modeHunk: no hunk expected when Windows read-only state matches")
		}
		return
	}

	// modeDrifted (status side).
	if modeDrifted(0o755, p) {
		t.Errorf("modeDrifted: 0755 recorded vs 0755 on disk must be false")
	}
	if err := os.Chmod(p, 0o644); err != nil {
		t.Fatal(err)
	}
	if !modeDrifted(0o755, p) {
		t.Errorf("modeDrifted: 0755 recorded vs 0644 on disk must be true (drift)")
	}
	if modeDrifted(0, p) {
		t.Errorf("modeDrifted: recorded mode 0 (unspecified) must never be drift")
	}
	if modeDrifted(0o755, filepath.Join(dir, "absent")) {
		t.Errorf("modeDrifted: a missing file must not be reported as mode drift")
	}

	// modeHunk (diff side): intended 0755 vs on-disk 0644 → a hunk.
	src, dst, ok := modeHunk(p, 0o755)
	if !ok {
		t.Fatalf("modeHunk: expected a hunk for intended 0755 vs on-disk 0644")
	}
	if src != "mode 0755" || dst != "mode 0644" {
		t.Errorf("modeHunk src=%q dst=%q, want 'mode 0755' / 'mode 0644'", src, dst)
	}
	if _, _, ok := modeHunk(p, 0o644); ok {
		t.Errorf("modeHunk: no hunk expected when intended mode == on-disk mode")
	}
	if _, _, ok := modeHunk(p, 0); ok {
		t.Errorf("modeHunk: no hunk expected for an unspecified intended mode (0)")
	}
}
