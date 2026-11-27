Task 1: done (uirf.1)
- Account card: `SidebarMenu::Account`, opened by `SidebarMenuMessage::OpenAccount`; entries through `SidebarMenuMessage::Account(AccountAction)`, which closes the card and then sends the entry's message (Log out sends `LogoutRequested`, so the warning flow is unchanged). Drawn by `main_view::account_card`, a `kit::dialog` like the Sort card, so Esc order is the sidebar cards' (after the switcher; keys test in `state::tests::sidebar_menu_flow`).
- No new iced API: only `button`, `row`, `column` and `kit` helpers already in use. The chevron is `Icon::SectionOpen` (no new icon).
- Gallery scenario `main-account-menu` added (fixture + names list).
- Gate and test notes in PROGRESS.md.
