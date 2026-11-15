Task 1: started
Task 1: complete (8d82443..eeef151, gate ok, security PASS, review PASS, evidence PROVEN)
  evidence: docs/plans/sidebar-rework.evidence/task-1.md
  notes: Low: removing a tag (tag None) decides from the cached is_favourite()/is_low_priority(), so a tag set on another device and not yet synced is left in place; optional fix: clear both. notice_text returns None for ActionKind::RoomTag: a later app task (L5) must add the arm. Mutation fails by timeout, not by a flag assertion. cargo deny not run (known failing advisories on main); check-commits.sh 342cd3b flags only the controller's plan:/ledger:/evidence: commits.
Task 2: started
Task 2: complete (b462bcb..8c659b7, gate ok, security PASS, review PASS, evidence PROVEN)
  evidence: docs/plans/sidebar-rework.evidence/task-2.md
  notes: SDK read-receipts filter matches num_unread > 0 || is_marked_unread, so a room marked unread also appears under Mentions (follows from K2): Task 7 hint text and Task 8 "Unreads first" ordering must not assume Mentions excludes marked rooms. Task 2 also added empty_label arms in crates/app/src/room_list.rs (exhaustive match, not in plan file list). Mutation (swap Favourite/LowPriority) fails the integration test by timeout. Task 7 table test could assert the Task 2 labels stay non-empty. cargo deny skipped (known advisories): track and list in the deny ignore config before release.
Task 3: started
Task 3: complete (3a0930c..7be68f1, gate ok, security PASS, review PASS, evidence PROVEN)
  evidence: docs/plans/sidebar-rework.evidence/task-3.md
  notes: no open issues. Optional: a test for a wrong-case value such as "name" (falls to the default). Mutation (unread_first always false) fails the round-trip test by assertion.
Task 4: started
Task 4: complete (e631152..0102cb3, gate ok, security PASS, review PASS, evidence PROVEN)
  evidence: docs/plans/sidebar-rework.evidence/task-4.md
  notes: (1) docs/sidebar.md must carry the K4 sentence (an older twicecat drops the two keys) in the app task that first writes them (Task 9 or 10). (2) crates/app/src/list_drop.rs: placeholder arms (drop onto Favourites/LowPriority = Refused, room already in one dropped on its built-in default = Nothing) must be revisited when the sections become real drop targets (L6: SetRoomTag, and default-section drop removes the tag); Task 9/10 acceptance tests should cover this. (3) list_drop.rs was edited beyond the plan file list (exhaustive match). (4) cosmetic: re-wrap the to_key doc comment in api/sidebar.rs when next touched. (5) the security auditor could not build without CC=clang-22: audits must set CC=clang-22 CXX=clang++-22.
Task 5: started
Task 5: complete (86b50f8..c1aa701, gate ok incl. cargo deny, security PASS, review PASS, evidence PARTIAL: all headless criteria proven; drag feel, real-window cursor and Esc during a live drag are on the human checkpoint in PROGRESS.md)
  evidence: docs/plans/sidebar-rework.evidence/task-5.md
  deviations for the human to confirm at the checkpoint (neither conflicts with the plan; both documented in docs/sidebar.md and docs/accessibility.md): (1) an OPEN folder cannot be reordered by dragging its head (sweeten picks the child whose centre is nearest the cursor); it moves with its card (Move up/down); a closed folder still drags by its head, spaces inside drag in the folder's column. (2) touch no longer reorders the rail (sweeten reads only mouse events; S5 removed the finger handling); the card does the same move.
  notes: app/gallery tests were written after the code and verified by mutation (both plan mutations recorded and reverted); cursor, 4.0/4.5 px and leave-the-scrollable tests were not mutation-checked. Low (audit): rail_generation flip relies on iced dropping widget state on a tree change: on any iced/sweeten upgrade keep the generation test and add a gallery check that no ghost is drawn after Esc. Low: RailReorder does not store the picked index (could refuse a Dropped whose index differs). rail_drag_subscription renamed list_drag_subscription. cargo deny passes (yanked clipboard-win 5.4.1 warning pre-existing).
