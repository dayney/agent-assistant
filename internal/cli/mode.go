package cli

import (
	"os"
	"runtime"
)

// modePermsDiffer compares the permission information that the current OS can
// actually represent. POSIX filesystems expose all permission bits, while Go's
// Windows implementation maps FileMode permissions to the read-only attribute:
// a writable file reports 0666 regardless of whether agentsync requested 0644
// or 0755. Comparing the raw bits on Windows would make every applied file look
// drifted immediately after apply.
func modePermsDiffer(want, got os.FileMode) bool {
	if runtime.GOOS == "windows" {
		return (want.Perm()&0o222 != 0) != (got.Perm()&0o222 != 0)
	}
	return want.Perm() != got.Perm()
}
