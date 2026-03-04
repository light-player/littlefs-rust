# lfs_dir_traverse: Control Flow Analysis & Restructuring Options

## C Structure Overview

The C `lfs_dir_traverse` uses an explicit stack to simulate recursion, with `goto` for control flow:

```
while (true) {
    {
        // --- "get next tag" block ---
        if (disk has more)      → read tag, continue
        else if (attrs left)    → consume attr, continue
        else                    → res=0, break   // exits while

        // --- filter ---
        if (mask mismatch)      → continue
        if (lfs_tag_id(tmask) != 0) {
            push frame, set cb=filter, continue  // get next tag
        }
    }

popped:
    // --- process tag ---
    if (begin/end filter)      → continue
    if (NOOP)                   → (nothing)
    else if (MOVE)              → push, recurse into move
    else if (USERATTRS)         → loop over attrs, cb each
    else                        → res = cb(data, tag, buffer)

    if (sp > 0) {
        pop frame
        goto popped
    } else {
        return res
    }
}
```

**Critical C behavior**: When we run out of tags (`break`), we exit the `while`. The `if (sp > 0)` that does the pop is *inside* the loop, so we never pop after `break`. The C must either (a) never run out with sp>0 in practice, or (b) have a return + pop path we haven't fully traced.

## Current Rust Structure

The Rust version uses a single `'outer` loop with:
- `from_pop` flag to distinguish "we just popped" vs "we got a new tag"
- `if sp > 0` at top → pop branch, set `from_pop = true`
- `else` → get next tag from disk or attrs
- When we run out: `continue 'outer` (simulates "need to pop")
- Push: save frame, `continue 'outer` (get next tag)

**Potential mismatch**: When we run out of tags, Rust does `continue 'outer``. The next iteration then takes the `sp > 0` branch and pops. So we *do* pop. The C `break` exits the loop and never pops. If the C truly has this behavior, our Rust "fix" (continuing to pop) might be correct and the C might rely on never hitting that case with sp>0. Or the C structure could be different (e.g. break only exits inner block in some dialect).

## Restructuring Options

### 1. Match C with `goto`-style labels (unsafe, macro, or inline)

Rust doesn't have `goto`, but we can approximate it:

```rust
fn lfs_dir_traverse(...) -> i32 {
    let mut state = TraverseState { ... };
    loop {
        // "get next tag" block
        let got_tag = state.get_next_tag(lfs);
        if !got_tag {
            break;  // C's break
        }
        if state.maybe_push_and_continue() {
            continue;
        }
        // popped: (fall through)
        if state.maybe_continue_filter() {
            continue;
        }
        let res = state.process_tag();
        if state.sp > 0 {
            state.pop();
            // goto popped = fall through to next iter, but we need to NOT get next tag
        } else {
            return res;
        }
    }
    // After break: C would return here. We need to pop if sp>0.
    state.pop_if_any()
}
```

The trick: when we "goto popped", we must process the popped tag, not fetch a new one. Our current Rust does this by looping: next iteration, `sp>0` → we take the pop branch and get `(tag, buffer)` from the frame. So we're effectively "at popped" with the right tag. The flow matches.

### 2. Extract local helper functions

Break the C into named steps that mirror the C:

```rust
fn lfs_dir_traverse(...) -> i32 {
    let mut ctx = TraverseCtx::new(...);

    loop {
        // Step 1: get next (tag, buffer)
        let next = ctx.get_next(lfs);
        let (tag, buffer) = match next {
            Next::Tag(t, b) => (t, b),
            Next::Exhausted => {
                if ctx.sp == 0 { return 0; }
                ctx.pop();
                continue;  // will re-enter as pop
            }
        };

        // Step 2: maybe push (filter recursion)
        if ctx.should_push(tag) {
            ctx.push(tag, buffer);
            continue;
        }

        // Step 3: popped - process
        if ctx.should_skip_filter(tag) { continue; }
        let res = ctx.dispatch(lfs, tag, buffer)?;
        if res != 0 && ctx.sp == 0 { return res; }
        if ctx.sp > 0 {
            ctx.pop();
            // Simulate goto popped: loop again but next iter we pop
            continue;
        }
        return res;
    }
}
```

This makes the C phases explicit and easier to compare.

### 3. State machine with explicit phases

```rust
enum TraversePhase {
    GetNextTag,
    ProcessTag { tag: lfs_tag_t, buffer: *const c_void },
    PopAndProcess,
}

fn lfs_dir_traverse(...) -> i32 {
    let mut phase = TraversePhase::GetNextTag;
    let mut stack = ...;

    loop {
        match &mut phase {
            GetNextTag => {
                // get tag or exhaust
                phase = ...;
            }
            ProcessTag { tag, buffer } => {
                // filter, NOOP/MOVE/USERATTRS/cb
                phase = ...;
            }
            PopAndProcess => {
                if sp == 0 { return res; }
                pop();
                phase = ProcessTag { tag: frame.tag, ... };
            }
        }
    }
}
```

Makes state transitions explicit, at the cost of moving tag/buffer through the enum.

### 4. Closest to C: two nested loops with explicit "popped" block

The C has two conceptual loops: (1) "get tag, maybe push" and (2) "process, maybe pop and retry". We can mirror that:

```rust
loop {
    // Outer: get next tag (or pop)
    let (tag, buffer) = if sp > 0 {
        pop_one();
        (frame.tag, frame.buffer)
    } else {
        match get_next_tag() {
            Some(t) => t,
            None => {
                if sp == 0 { return 0; }
                pop_one();
                continue;  // now sp>0, will pop next iter
            }
        }
    };

    // Filter (maybe push)
    if should_push(tag) {
        push(tag, buffer);
        continue;
    }

    // "popped:" block - process
    if skip_by_filter(tag) { continue; }
    let res = do_cb(tag, buffer);
    if res != 0 && sp == 0 { return res; }
    if sp > 0 {
        pop_one();
        continue;
    }
    return res;
}
```

### 5. Add a "run_callback" helper to isolate the bug

Without restructuring, add a small helper so the exact callback invocation matches C:

```rust
fn dispatch_tag(
    cb: CbType,
    data: *mut c_void,
    tag: lfs_tag_t,
    buffer: *const c_void,
    diff: i16,
) -> i32 {
    let out_tag = tag.wrapping_add(lfs_mktag(0, diff as u32, 0));
    unsafe { cb(data, out_tag, buffer) }
}
```

Then the main loop only decides *when* to call this. Easier to add logging and to compare with C.

## Recommendation

1. **First**: Add minimal instrumentation (e.g. `dispatch_tag` + a cfg(test) counter) to confirm whether the callback is ever invoked for SUPERBLOCK during format.

2. **Then**: If the callback is never invoked, the bug is in the push/pop/continue flow. Option 2 (local helpers) or 4 (two-loop structure) would make that flow easier to audit against the C.

3. **Long term**: A small state machine (Option 3) or the two-loop layout (Option 4) would keep the control flow close to the C and make future translation of MOVE and USERATTRS simpler.
