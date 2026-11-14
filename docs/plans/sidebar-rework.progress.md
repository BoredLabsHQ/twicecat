Task 1: started
Task 1: complete (8d82443..eeef151, gate ok, security PASS, review PASS, evidence PROVEN)
  evidence: docs/plans/sidebar-rework.evidence/task-1.md
  notes: Low: removing a tag (tag None) decides from the cached is_favourite()/is_low_priority(), so a tag set on another device and not yet synced is left in place; optional fix: clear both. notice_text returns None for ActionKind::RoomTag: a later app task (L5) must add the arm. Mutation fails by timeout, not by a flag assertion. cargo deny not run (known failing advisories on main); check-commits.sh 342cd3b flags only the controller's plan:/ledger:/evidence: commits.
