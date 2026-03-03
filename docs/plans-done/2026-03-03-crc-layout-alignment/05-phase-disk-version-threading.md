# Phase 5: Thread disk_version through commit callers

## Scope of phase

Wire MountState.disk_version (or format's DISK_VERSION) through to all commit callers. Ensure relocatingcommit, fs_gc, and format pass the correct disk_version.

## Code organization reminders

- disk_version flows from mount state or format context
- dir_relocatingcommit receives dir from fetch; fetch doesn't store disk_version on MdDir

## Implementation details

### 1. dir_relocatingcommit

Currently called from dir_orphaningcommit. dir_orphaningcommit is called from many places. We need disk_version at the point of call. Options:
- Add disk_version to dir_relocatingcommit's params — caller must provide
- dir_orphaningcommit already receives various ctx — add disk_version to a context struct

Trace callers of dir_relocatingcommit: dir_orphaningcommit. Callers of dir_orphaningcommit: fs_gc, mkdir, create, etc. — all have access to MountState or similar. The fs operations go through LittleFs which has state.

Add `disk_version: u32` to dir_relocatingcommit. Add to dir_orphaningcommit. Callers obtain from state.disk_version.

### 2. dir_commit_append callers

dir_commit_append is called from dir_relocatingcommit. So dir_relocatingcommit receives disk_version and passes to dir_commit_append.

### 3. dir_compact callers

dir_compact is called from dir_splittingcompact, which is called from dir_relocatingcommit and dir_split. dir_relocatingcommit has disk_version. dir_split is called from splittingcompact and elsewhere — ensure the chain passes disk_version. dir_splittingcompact calls dir_compact — add disk_version to dir_splittingcompact, from dir_relocatingcommit.

### 4. fs_gc

fs_gc calls dir_orphaningcommit. Add disk_version param to dir_orphaningcommit. fs_gc has state.disk_version.

### 5. Full caller chain

- LittleFs::fs_gc: state.disk_version → dir_orphaningcommit
- dir_orphaningcommit: disk_version → dir_relocatingcommit
- dir_relocatingcommit: disk_version → dir_commit_append, dir_splittingcompact
- dir_splittingcompact: disk_version → dir_compact
- dir_split: disk_version → dir_compact (dir_split calls dir_compact)
- Other callers of dir_orphaningcommit (mkdir, etc.): need disk_version from their context

Audit all dir_orphaningcommit call sites and ensure they pass disk_version.

## Validate

```bash
cd lp-littlefs && cargo test
cargo fmt
```
