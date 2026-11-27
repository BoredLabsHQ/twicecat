Task 1: done (uirf.1)
- Account card: `SidebarMenu::Account`, opened by `SidebarMenuMessage::OpenAccount`; entries through `SidebarMenuMessage::Account(AccountAction)`, which closes the card and then sends the entry's message (Log out sends `LogoutRequested`, so the warning flow is unchanged). Drawn by `main_view::account_card`, a `kit::dialog` like the Sort card, so Esc order is the sidebar cards' (after the switcher; keys test in `state::tests::sidebar_menu_flow`).
- No new iced API: only `button`, `row`, `column` and `kit` helpers already in use. The chevron is `Icon::SectionOpen` (no new icon).
- Gallery scenario `main-account-menu` added (fixture + names list).
- Gate and test notes in PROGRESS.md.
Task 1: complete (b523e1b..866a209, gate ok, security PASS, review PASS, evidence PROVEN)
  evidence: docs/plans/ui-review-fixes.evidence/task-1.md
  notes: Low: Log out runs at once when no warning is needed (as before, now two clicks); Voice & audio entry not asserted end to end in state tests; AccountAction lives in screens::main_view and sidebar_menu imports it (layering); "Account" fallback label duplicated. Card is a centred dialog like the Sort card, not a drop-down under the button - human to confirm. Own avatar is placeholder initials. Real window = human.
Task 2: started
