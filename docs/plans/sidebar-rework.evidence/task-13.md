# Evidence: Task 13 (documents, deferred list and checkpoint)

Commits: 7a16ae6 and the follow-up 0b22588 (diff base 3ed66f5). Collected by the controller at 0b22588 with CC=clang-22 CXX=clang++-22.

- `cargo test -p twicecat-app --test docs_sidebar`: `test result: ok. 16 passed; 0 failed`.
- `grep -n "homeserver" docs/sidebar.md` shows the tag sentences (lines 89, 170, 206, 208, 215 at 7a16ae6; the follow-up reworded two paragraphs).
- The full plan gate at 0b22588: `cargo fmt --all --check`, clippy with and without `--all-features`, `cargo check --workspace --lib --bins --locked`, `scripts/check-app-deps.sh`, `scripts/check-core-api.sh`, `bash scripts/check-font-assets.sh`, `bash scripts/check-no-webrtc-download.sh` and `cargo deny check` all exit 0; `cargo test --workspace`: 3046 passed, 0 failed.
- The sidebar rework human checkpoint in PROGRESS.md has 11 items, all unticked (0 ticked).
- Reviewer's mutation: removing every "Mark as low priority" from docs/sidebar.md made `every_entry_of_every_options_card_is_in_sidebar_md` and `sidebar_md_names_the_menu_entries_the_app_has` fail; restored.
- Not proven (human checkpoint): everything the checkpoint lists (real rendering, drags, overlays, tag round trips with Element).
