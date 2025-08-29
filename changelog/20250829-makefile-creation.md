# Makefile Creation for spkrd

## Task Specification
Create a top-level Makefile with support for 'all', 'clean', and 'install' targets.

### Install Requirements:
- Configurable destination directory via DSTDIR (default: /usr/local)
- Binary installation in $DSTDIR/bin
- Binary name configurable via PROGRAM (default: 'spkrd')
- FreeBSD init script installation in $DSTDIR/etc/rc.d
- Init script should use standard rc.conf variables

## Requirements Clarification
- Build target configurable via BUILD variable (default: release)
- Clean target: main project only using 'cargo clean'
- Init script: Follow FreeBSD rc-scripting docs, model after 'mumbled' example
- Install target depends on all target
- Use reasonable permissions for system daemon
- User configures port, device, retry timeout through explicit flags in spkrd_flags (not separate rc.conf vars)
- Include examples of flag usage directly in the rc.d script comments

## Files Modified
- **Makefile**: Top-level build system with configurable DSTDIR, PROGRAM, BUILD variables
- **rc.d/spkrd**: FreeBSD init script with comprehensive flag usage examples
- **README.md**: Added system-wide installation and service configuration documentation

## Implementation Decisions
- Used standard FreeBSD rc.subr patterns based on mumbled example
- Included detailed usage examples in init script comments
- Made all key variables configurable via Makefile
- Tested both release and debug build modes

## Current Status
- Implementation completed successfully
- All Makefile targets tested and working
- Init script follows FreeBSD conventions with embedded documentation