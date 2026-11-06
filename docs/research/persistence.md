# Persistence: SQLite, a single file, or an embedded store?

**Researched:** 2026-11-06, for rusqlite 0.41.1 (SQLite 3.54.1 bundled), redb 4.5.0, serde_json 1.0.154, ron 0.12.3,
rmp-serde 1.3.1, toml 1.1.8, rustc 1.97.1, on Windows 11, Ryzen 9 3950X (32 threads), Samsung 970 EVO Plus NVMe, NTFS.
**Question (owner, 2026-11-06):** "should we consider using a db like sqlite at this point?"
**Scope:** SAV-01..03, SET-01/02, PRG-06, §6.2 and §7.2. The marks are the same as in
[simulation-architecture.md](simulation-architecture.md): **Measured** means run on this machine for this document,
**Verified** means read in the crate source or official docs (linked), and **Inference** is my reasoning. Context7 wasn't
available. The sources are the crate sources in the cargo registry and the official pages under [Sources](#sources).

## Bottom line

- **Nothing measured comes close to the budget, so speed and RAM don't decide this.** With 5,000 memory records, every
  option loads in 1 to 22 ms, saves in 1 to 20 ms, and holds 0.3 to 1.7 MB more RAM than the data itself. The headroom
  is about 50 MB of RAM and 0.32 of a core. Measured.
- **A database's real advantage is the cost of one change.** SQLite commits one change durably in 1.1 ms and redb in
  1.4 ms, whatever the size. A single file is rewritten whole: 3 to 13 ms at 5,000 records, and 23 to 90 ms at 50,000.
  Even 90 ms every 5 seconds is 0.06 % of the machine. Measured, and the CPU share is arithmetic.
- **SQL querying doesn't help here.** The Mind scores memories in `pet-core`, which has no IO, so memory has to live in
  RAM anyway. An in-RAM scan for "20 newest strong *petted* memories" takes 0.012 ms. The same query through SQLite's
  index takes 0.027 ms. Measured.
- **None of the options reliably detects a flipped bit by default.** Over 20 one-bit or one-digit flips: JSON, RON and TOML loaded
  wrong data 20 times out of 20, MessagePack 8, SQLite 4 (even after `PRAGMA quick_check`), and redb 14 with a default
  open. redb with `set_check_integrity_on_open(true)` loaded wrong data 0 times. So a single file needs a checksum.
  `crc32fast` is already in the app's tree. Measured.
- **Recommendation:** a single, pretty-printed JSON file, protected by a checksum, written atomically on a writer
  thread, with rolling copies and a chain of pure migration functions. Revisit SQLite if memory must outgrow RAM, or the
  file passes about 10 MB (about 50,000 records). [Recommendation](#recommendation).

## Terms

| Term | Meaning |
|---|---|
| **fsync / flush** | Asking the OS to put written bytes on the disk itself, not just in its cache. On Windows it is `FlushFileBuffers`. Rust's `File::sync_all` calls it. |
| **Atomic replace** | Write a new file, flush it, then rename it over the old one, so a crash leaves the old file or the new one, never half of each. |
| **Torn write** | A file that is half old and half new after a crash mid-write. |
| **Journal / WAL** | SQLite's crash safety. Changes go to a side file first (rollback journal or Write-Ahead Log) so a crash can be undone or replayed. |
| **Checkpoint** | SQLite copying the WAL back into the main file. |
| **Migration** | Code that turns data saved by an older version of the app into the current shape. |
| **DTO** | Data Transfer Object: a plain struct shaped for the file, kept separate from the domain types. |
| **Snapshot vs delta** | Sending the writer the whole current state, or only what changed. Snapshots can be dropped if a newer one arrives. Deltas can't. |
| **Private working set** | Task Manager's "Memory" column, and the measure in §6.2. |
| **Commit** | Memory the process has reserved, whether or not it is in RAM now. "Peak commit" is the most it ever reserved. |

## How it was measured

A throwaway workspace in `%TEMP%\desktop-pet-storage-spike`, deleted afterwards, with the repo's release profile
(`codegen-units = 1`, `lto = "thin"`) and linker (`rust-lld`). Each backend is a separate binary over one shared model
crate:

- **The dataset.** One Character holds N memory records: id, time, kind, an optional subject, strength and valence.
  It also holds N/5 XP entries, the current Form and the unlocked Forms. Two more Characters are small. The settings
  and position are included. In RAM, N = 5,000 takes 0.57 MB of private working set, and N = 50,000 takes 5.5 MB.
- **Files.** Saving means serialising, writing `save.tmp`, `sync_all`, then `std::fs::rename`. That is
  `MoveFileExW(MOVEFILE_REPLACE_EXISTING)` in std's source. Loading means `fs::read` followed by deserialising.
- **SQLite.** WAL mode, `synchronous=FULL`, one table per collection, prepared statements, and one transaction per
  save.
- **redb.** Native tuple values with no serde, and the default `Durability::Immediate`.
- **"One change"** is what a ProgressChanged causes: one memory record, one XP entry and a Form update, made
  durable. A file has to be rewritten whole for it.
- **Timing.** The median of 11 runs in the same process, with the first run shown alongside it. Every load is checked
  against what was saved. The fsync times depend on this NVMe disk.
- **RAM.** `K32GetProcessMemoryInfo` with `PROCESS_MEMORY_COUNTERS_EX2`. "Held" is the private working set after load,
  with the store still open, minus the same dataset held with no store. "Peak" is the peak commit of a fresh process
  doing one save or one load, minus its start, so it includes the data.
- **Size.** Release `.exe` bytes, minus a baseline binary that has the model, the probe and no storage. That baseline
  is 215 KB, and an empty `main` is 112 KB.
- **Build time, standalone.** A clean `cargo build --release` of each binary into its own target directory, median of
  5 runs, minus the baseline. The baseline is 8.8 s, and single runs spread by up to ±6 s.
- **Build time in the app.** A clean release build of a copy of this repo, with the crate added to `pet-app`, run once
  per variant. `cargo tree` confirmed the added crate was in the build graph. Two builds of the unchanged copy took
  341 s and 266 s, so anything under about 75 s is lost in that noise. The crates compile in parallel with Bevy. The release `.exe` is 40.6 MB.
- **Corruption.** The 5,000-record save was damaged three ways: truncated to half, 4 KiB zeroed in the middle, and one
  digit (text) or bit (binary) flipped at 20 evenly spaced places, one copy each. Each copy was then loaded.

## Comparison (measured, N = 5,000 unless marked)

| | JSON (serde_json, pretty) | RON (pretty) | MessagePack (rmp-serde) | TOML | SQLite (rusqlite bundled) | redb |
|---|---|---|---|---|---|---|
| Already in the app's tree | no (`serde_json`, `itoa`, `zmij`) | **yes** (via `bevy_asset`) | no (`rmp-serde`, `rmp`) | **yes** (`pet-content`) | no (14 crates incl. 5 build tools, + C source) | no (no dependencies) |
| Added `.exe` size, standalone | +187 KB | +376 KB | +230 KB | +412 KB | **+1,760 KB** | **+1,406 KB** |
| Added clean build, standalone | +4.5 s | +16.4 s | +12.7 s | +7.9 s | +18.0 s | +26.6 s |
| Added clean build, in the app | within noise (274 s) | 0 (already built) | not run | 0 (already built) | within noise (327 s) | within noise (286 s) |
| Needs a C compiler | no | no | no | no | **yes** (MSVC `cl.exe`, built here) | no |
| File size | 1.09 MB | 1.53 MB | 0.40 MB | 0.66 MB | 0.63 MB (+ `-wal`, `-shm` while open) | 0.79 MB (0.34 compacted) |
| Load everything | 3.2 ms | 13.4 ms | 1.6 ms | 21.5 ms | 2.6 ms | 1.2 ms |
| Save everything | 5.8 ms | 9.8 ms | 3.3 ms | 12.3 ms | 19.9 ms | 8.4 ms |
| One change, durable | 6.0 ms (whole file) | 9.6 ms | 3.4 ms | 12.6 ms | **1.1 ms** (WAL+FULL); 5.0 ms (DELETE+FULL) | **1.4 ms** |
| Backup copy | 1.3 ms (`fs::copy`) | 1.6 ms | 0.7 ms | 1.3 ms | 9.2 ms (`VACUUM INTO`) | 13.3 ms (close, copy, reopen) |
| RAM held over the data | +0.9 MB | +0.7 MB | +0.3 MB | +1.7 MB | +1.5 MB | +1.3 MB |
| Peak commit, one save / one load | 4.1 / 2.3 MB | 4.1 / 2.7 MB | 1.0 / 1.0 MB | 4.7 / **23.0 MB** | 0.9 / 1.7 MB | 0.6 / 1.5 MB |
| N = 50,000: load / save / one change | 35 / 51 / 51 ms | 140 / 88 / 90 ms | 20 / 23 / 23 ms | not run | 33 / 308 / 1.3 ms | 14 / 83 / 1.9 ms |
| N = 50,000: peak commit save / load | 24 / 19 MB | 24 / 24 MB | 6 / 12 MB | not run | 2.4 / 11 MB | 3.7 / 11 MB |
| Readable in an editor | yes | yes | no | yes | with a tool (`sqlite3`, DB Browser) | no (code only) |

**Corruption (measured).** "Wrong" means the load succeeded with wrong data and no error.

| | Truncated to half | 4 KiB zeroed | 20 single flips |
|---|---|---|---|
| JSON / RON / TOML | error | error | **20 wrong** |
| MessagePack | error | error | 12 errors, **8 wrong** |
| SQLite (with or without `quick_check`) | error | error | 3 errors, **4 wrong**, 13 unaffected |
| redb, default open | error | **panic** (`unreachable`) | 4 errors or panics, **14 wrong**, 2 unaffected |
| redb, `set_check_integrity_on_open(true)` | error | error | 18 errors, **0 wrong**, 2 unaffected |

A checksum over the whole file turns every one of those wrong loads into an error. That is the reason the checksum is
part of the recommendation. Inference: a CRC-32 misses a random corruption about once in 4 billion.

**What the CPU numbers mean.** The budget is 1 % of 32 threads, which is 0.32 of one core, all the time. Saving after
at most one change every 5 seconds:

- JSON at 5,000 records: 6 ms ÷ 5 s = 0.12 % of one core, which is 0.004 % of the machine.
- RON at 50,000 records: 90 ms ÷ 5 s = 0.06 % of the machine.

Most of each save is waiting for the flush, not computing. JSON's serialise step alone takes 2.1 ms. Arithmetic on
measured numbers.

## SQLite through `rusqlite` (`bundled`)

- **What `bundled` does.** It compiles SQLite's C source, a 9.6 MB `sqlite3.c`, into the binary with the `cc` crate.
  Here cc-rs found Visual Studio 2022 Community (MSVC 14.36/14.37) without a developer shell, and the build worked.
  Measured.
- **A C compiler would be new.** The app compiles no C for Windows today: `cc` is in `Cargo.lock` only for
  `android-activity`. Measured with `cargo tree -i cc`. Inference: the MSVC Rust target already needs the Visual Studio
  Build Tools for its libraries, and `cl.exe` comes in the same workload, so in practice only CI images need checking.
  The README calls `bundled` "a good option for cases where linking to SQLite is complicated, such as Windows".
  [rusqlite README](https://github.com/rusqlite/rusqlite/blob/v0.41.1/README.md)
- **Crash safety.** With WAL and `synchronous=FULL`, SQLite is "ACID". With `NORMAL`, it is "always consistent" but "might
  roll back following a power loss". [pragma.html](https://www.sqlite.org/pragma.html#pragma_synchronous) Measured:
  one change took 1.1 ms with FULL and 0.05 ms with NORMAL. SQLite has no page checksums by default, so a flipped bit can
  load as wrong data (4 of 20). The optional [checksum VFS](https://www.sqlite.org/cksumvfs.html) exists, but I didn't
  check whether rusqlite can use it. Not verified.
- **SAV-01 says "one local save file".** In WAL mode, `save.db-wal` and `save.db-shm` sit beside the file while it is
  open. Rollback-journal mode (`journal_mode=DELETE`) keeps one file, apart from a short-lived journal, at 5.0 ms per
  change. Measured.
- **Backups.** `VACUUM INTO` writes "a consistent snapshot" while the database is in use, and flushes it when
  `synchronous` is NORMAL or FULL. [lang_vacuum.html](https://www.sqlite.org/lang_vacuum.html#vacuuminto) The backup API
  (`Connection::backup`, behind the `backup` feature) does the same. Measured: 9.2 ms and 11.1 ms.
- **Migrations.** `PRAGMA user_version` is an integer that "SQLite makes no use of", there for the app to use. A
  migration is a list of SQL scripts run in a transaction. [rusqlite_migration 2.6.0](https://docs.rs/rusqlite_migration/2.6.0)
  packages exactly that. SQLite's `ALTER TABLE` is limited, so reshaping a table means building a new one and copying
  ([lang_altertable](https://www.sqlite.org/lang_altertable.html#otheralter)).
- **Export and import.** Export with `VACUUM INTO 'export.db'`. Import means opening the file, checking `user_version`
  and `integrity_check` (6 ms here), then `Connection::restore` or a file swap.
- **Threads.** `Connection` is `Send` but not `Sync` ([lib.rs](https://github.com/rusqlite/rusqlite/blob/v0.41.1/src/lib.rs)).
  So the writer thread owns it.

### Why not `sqlx` or `diesel`

- **`sqlx` 0.9.0** is async. Almost every call "will panic without at least one runtime feature enabled", meaning
  Tokio or async-std ([docs](https://docs.rs/sqlx/0.9.0/sqlx/)). That is a second scheduler beside Bevy's, just to
  write one file.
- **`diesel` 2.3.14** is a synchronous ORM: a query builder plus code generated from the schema. It brings the most
  generated code and the slowest builds for six small tables. Inference.
- **Neither can share a build with rusqlite 0.41.** `sqlx-sqlite` pins `libsqlite3-sys` below 0.38, and diesel pins it
  below 0.39. rusqlite 0.41.1 needs 0.39.1, and `libsqlite3-sys` declares `links = "sqlite3"`, so only one version may
  be in a build. Verified on [docs.rs features pages](https://docs.rs/crate/sqlx-sqlite/0.9.0/features) and in the
  crate manifest.
- **Neither adds anything this app needs over `rusqlite`.**

## A single serde file

- **Crash safety.**
  - Write `save.tmp`, `sync_all`, then rename it over `save.json`.
  - std's `rename` is `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING`, without `MOVEFILE_WRITE_THROUGH`. Verified in
    std's `sys/fs/windows.rs`.
  - Microsoft's page for `MoveFileExW` doesn't promise the rename is atomic. For "document-like data" it recommends
    writing a new file, then replacing the original with
    [`ReplaceFileW`](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-replacefilew)
    ([Alternatives to TxF](https://learn.microsoft.com/en-us/windows/win32/fileio/deprecation-of-txf)).
  - Inference: either way, the flush before the rename means a power cut leaves the old file or the new one. Without
    write-through, the last save may be lost. A checksum and the rolling copies cover what's left.
  - A virus scanner or the search indexer can hold the file open briefly, so the rename needs a short retry. Inference.
- **Rolling backups.** A backup is a plain `fs::copy`, 1.3 ms. Inference: copying on every save would make all N copies
  seconds apart. Copy at launch, after a successful load, and once a day instead, so a bad save that loaded fine is
  still recoverable.
- **Corruption must never lose the Pet (SAV-02).**
  - On load, try `save.json`, then `save.tmp` if its checksum is good, then the backups, newest first.
  - Never overwrite a file that failed. Rename it to `save.corrupt-<time>.json` and log it.
  - A text file can be repaired by hand, and the binary formats can't. A hand edit breaks the checksum, though. One way
    to allow it: setting `"crc32": "edited"` tells the loader to accept the file and log that, and the next save writes
    a real checksum again.
- **Where the checksum goes.** Put it inside the JSON, so the file stays valid JSON for editors and `jq`:
  `{"format": "desktop-pet-save", "version": N, "crc32": "…", "data": {…}}`. The CRC covers the exact bytes of `data`,
  which `serde_json`'s `raw_value` feature hands over without re-serialising. Inference, not built in the spike.
- **Migrations.**
  - A `version` field comes first. The loader reads only that field, then deserialises into the matching DTO
    (`SaveV1`, `SaveV2`, ...).
  - It then runs a chain of pure functions, such as `fn v1_to_v2(SaveV1) -> SaveV2`.
  - Each step is one Given-When-Then test over a checked-in fixture file of the old version.
  - Adding a field needs no new version: `#[serde(default)]` fills it in when it is missing.
- **Export and import.** The save file is the export. Import goes through the same load path: checksum, then parse,
  then migrate. If that passes, back up the current save and replace it atomically.
- **The formats.**
  - **JSON** is the fastest of the text formats here. Every editor and `jq` can read it.
  - **RON** is already compiled into the app, so it adds nothing to the build. It writes Rust enums naturally. It is 4
    times slower to load (13 ms against 3 ms) and its files are 40 % larger.
  - **TOML** is already used for Packs, but it is the slowest, and its load peaks at 23 MB for 5,000 records.
  - **MessagePack** is the fastest and smallest, but it can't be read and it hides 8 flips in 20.
  - **bincode** is out: its README says "Bincode is now unmaintained", and 3.0.0 is a one-line release
    ([readme](https://static.crates.io/readmes/bincode/bincode-3.0.0.html)). It names postcard as an alternative.

## redb

- **What it is.** A pure-Rust B-tree store with no dependencies, "crash-safe by default". Every page has an XXH3-128
  checksum. A commit is one fsync ("1PC+C"), and a partly written commit is rolled back on open after a crash
  ([design.md](https://github.com/cberner/redb/blob/v4.5.0/docs/design.md)).
- **The catches, measured.**
  - **A plain open trusts the file after a clean shutdown.** 14 of 20 flips loaded wrong data, and zeroed pages
    panicked inside redb. `set_check_integrity_on_open(true)` fixes that: 0 wrong, and a clean `Corrupted` error. Its
    doc says "a plain open may panic". Verified in `db.rs`.
  - **The file can't be copied while it is open.** An open `Database` holds a byte-range lock, so `fs::copy` fails with
    error 33. A backup means close, copy and reopen (13 ms), or an export written in code.
  - **The cache.** The default cap is 1 GiB (`Builder::new`, `db.rs`). It only filled to about 1 MB here, but set it
    explicitly.
- **Stability.** The releases break often:
  - 3.0 (2025-08) dropped file format v2, and only 2.6 could upgrade it.
  - 4.0 (2026-04) removed the `Legacy` type.
  - 3.1.3 and 4.0 fixed "a data loss bug".
  - 5.0 is already in the changelog.

  A save that must last years would have to follow those upgrades ([CHANGELOG](https://github.com/cberner/redb/blob/v4.5.0/CHANGELOG.md)).
- **Migrations.** There is no schema. Tables are typed in Rust, so changing a value's shape means a new table, copied
  inside one write transaction, plus a version key in a `meta` table.
- **Readability.** None. The file can only be read through code.

## How each fits the clean architecture

These points hold whichever backend is chosen:

- **`pet-core` stays free of serde and IO**
  ([ADR-0002](../adr/0002-simulation-in-its-own-ecs-crate.md), §7.2). So every backend needs DTOs in the adapter,
  mapped to and from the domain types. For the file option, the versioned DTOs are the migration mechanism itself. For
  SQLite and redb, the mapping is row code instead (about 120 lines each in the spike).
- **The write path.**
  1. A `pet-app` system reads `ProgressChanged` and sends the writer thread a message over `std::sync::mpsc`.
  2. The writer blocks on `recv`, with no polling, and writes.
  3. On `AppExit` it flushes and is joined.
- **That system must run every update.** Bevy drops a message after two updates. While Hidden, messages stay buffered
  (ADR-0002).
- **Snapshot or delta.**
  - The file option sends a snapshot. A clone of the 5,000-record dataset takes 0.52 ms. Measured.
  - The writer can drain the channel and keep only the newest snapshot, so a burst of changes costs one write.
  - A database needs ordered deltas, which can't be dropped. Either the domain describes every change, or the writer
    works out the difference itself. That is more code in the adapter, and more ways to diverge from what's in RAM.
- **Where the adapter lives.** The adapter could be its own crate, `pet-save`, like `pet-content`: pure Rust plus
  serde, with tests that build without Bevy. `pet-app` would only wire it up.

## Recommendation

**Don't use SQLite now.** Use one JSON file per install. It holds the settings, the last position and every Character's
Progress, and has these parts:

1. A JSON envelope, `{"format", "version", "crc32", "data"}`, with the CRC-32 over `data`. `crc32fast` is already in
   the tree through `png`.
2. Atomic replace: write the tmp file, `sync_all`, then `rename` with a short retry, which is what was measured.
   `ReplaceFileW` is Microsoft's suggested route, but it fails when no target exists, so the first save still needs
   `rename`. Inference: it wasn't measured.
3. Writes happen on one writer thread that keeps only the newest snapshot. They are debounced, and flushed on exit.
4. Rolling copies at launch and daily (keep 5 to start with), plus quarantine of any file that fails to load.
5. A `version` field and versioned DTOs with a chain of pure migration functions, each tested against a fixture.
6. Export is a copy of the file. Import is the load path, followed by an atomic replace.

**Why.**

- It meets every budget with a wide margin. It is the only option you can read and fix by hand, and its export is the
  save file itself.
- It adds one small crate, `serde_json` (+187 KB on a 40.6 MB release exe, and no measurable change to a clean build), and no C compiler.
- The cost that grows is the whole-file rewrite, about 1 ms per 1,000 records. It stays under 0.1 % of the machine
  until well past 50,000 records.

**Move to SQLite when** memory has to be queried without loading it into RAM, which means it is no longer in the domain,
or when the file passes about 10 MB. The DTO and writer-thread seam stays the same, so the swap is limited to the
adapter. redb's measured strengths (1.2 ms load, 1.4 ms change) don't outweigh the file locking, the opaque file and
the release churn for a save that must outlive app versions.

## Open decisions for the owner

1. **JSON or RON.**
   - JSON is faster (3 ms against 13 ms to load) and readable by every tool, but adds `serde_json`.
   - RON adds nothing to the build and writes enums the Rust way.
2. **The save debounce, against PRG-06 "XP is never lost".**
   - A power cut can lose whatever wasn't flushed yet. With a 5-second debounce, that is up to 5 seconds of XP.
   - Saving on every change costs 6 ms each.
   - A clean exit always flushes.
3. **How large memory may grow.** Forgetting weak records, or decaying them lazily from `(strength, at_ms)` with no
   writes, keeps the file bounded. This is a domain rule, and it decides when a database would ever pay off.
4. **How many rolling copies to keep, and when to take them:** at launch, daily, before a migration or before an import.
5. **Whether persistence gets its own crate (`pet-save`), or a module in `pet-app`.** This decision may deserve an ADR,
   beside ADR-0001.
6. **What the app does when every copy fails.** Starting a new egg must never overwrite the broken files. Should it
   show a message, or open the folder?

## Sources

- SQLite: [pragma synchronous / user_version / cache_size](https://www.sqlite.org/pragma.html),
  [VACUUM INTO](https://www.sqlite.org/lang_vacuum.html#vacuuminto), [WAL](https://www.sqlite.org/wal.html),
  [atomic commit](https://www.sqlite.org/atomiccommit.html), [checksum VFS](https://www.sqlite.org/cksumvfs.html).
- [rusqlite 0.41.1](https://docs.rs/rusqlite/0.41.1/rusqlite/) and its
  [README](https://github.com/rusqlite/rusqlite/blob/v0.41.1/README.md). libsqlite3-sys 0.39.1 bundles SQLite 3.54.1
  (`sqlite3/sqlite3.h`). [rusqlite_migration 2.6.0](https://docs.rs/rusqlite_migration/2.6.0).
- [redb 4.5.0](https://docs.rs/redb/4.5.0/redb/), with its
  [design.md](https://github.com/cberner/redb/blob/v4.5.0/docs/design.md) and
  [CHANGELOG](https://github.com/cberner/redb/blob/v4.5.0/CHANGELOG.md). `Builder` is in `src/db.rs`.
- [sqlx 0.9.0](https://docs.rs/sqlx/0.9.0/sqlx/), [sqlx-sqlite features](https://docs.rs/crate/sqlx-sqlite/0.9.0/features),
  [diesel 2.3.14 features](https://docs.rs/crate/diesel/2.3.14/features).
- [serde_json 1.0.154](https://docs.rs/serde_json/1.0.154), [ron 0.12.3](https://docs.rs/ron/0.12.3),
  [rmp-serde 1.3.1](https://docs.rs/rmp-serde/1.3.1), [toml 1.1.8](https://docs.rs/toml/1.1.8),
  [bincode 3.0.0 readme](https://static.crates.io/readmes/bincode/bincode-3.0.0.html).
- Windows: [MoveFileExW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw),
  [ReplaceFileW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-replacefilew),
  [Alternatives to TxF](https://learn.microsoft.com/en-us/windows/win32/fileio/deprecation-of-txf). std's `fs::rename`
  is in `library/std/src/sys/fs/windows.rs` of rustc 1.97.1.
