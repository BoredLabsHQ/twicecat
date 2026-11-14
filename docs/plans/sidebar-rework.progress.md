Task 1: started
Task 1: complete (8d82443..eeef151, gate ok, security PASS, review PASS, evidence PROVEN)
  evidence: docs/plans/sidebar-rework.evidence/task-1.md
  notes: Low: removing a tag (tag None) decides from the cached is_favourite()/is_low_priority(), so a tag set on another device and not yet synced is left in place; optional fix: clear both. notice_text returns None for ActionKind::RoomTag: a later app task (L5) must add the arm. Mutation fails by timeout, not by a flag assertion. cargo deny not run (known failing advisories on main); check-commits.sh 342cd3b flags only the controller's plan:/ledger:/evidence: commits.
Task 2: started
Task 2: complete (b462bcb..8c659b7, gate ok, security PASS, review PASS, evidence PROVEN)
  evidence: docs/plans/sidebar-rework.evidence/task-2.md
  notes: SDK read-receipts filter matches num_unread > 0 || is_marked_unread, so a room marked unread also appears under Mentions (follows from K2): Task 7 hint text and Task 8 "Unreads first" ordering must not assume Mentions excludes marked rooms. Task 2 also added empty_label arms in crates/app/src/room_list.rs (exhaustive match, not in plan file list). Mutation (swap Favourite/LowPriority) fails the integration test by timeout. Task 7 table test could assert the Task 2 labels stay non-empty. cargo deny skipped (known advisories): track and list in the deny ignore config before release.
Task 3: started
