# Evidence — Task 1: account menu card
Commit: 866a209
Environment: CC=clang-22 CXX=clang++-22, cargo test -p twicecat-app (headless tiny-skia gallery, no real window). Screenshot: task-1-main-account-menu.png (twicecat theme, comfortable, 1280x800; other 7 variants written to the scratchpad, not kept).

## One account button (avatar, label, chevron) opening a card: Settings, Voice & audio, Sessions, divider, Log out (danger). Not a pick_list; Ctrl+wheel never sends a message.
Status: PROVEN (rendering of the real window is human-only)
```console
$ TWICECAT_GALLERY_DIR=$S/g TWICECAT_GALLERY_ONLY=main-account cargo test -p twicecat-app --test ui_gallery write_the_gallery -- --ignored
test write_the_gallery ... ok   (8 main-account-menu-*.png written)
$ cargo test -p twicecat-app account
test screens::main_view::tests::the_account_card_lists_four_entries_with_log_out_last ... ok
test screens::main_view::tests::the_account_card_builds ... ok
test result: ok. 39 passed; 0 failed
$ grep -n "pick_list" crates/app/src/screens/main_view.rs
8: import; 420: notification_menu (room notification mode, unrelated); 1046-1048: doc comments only
```
Image: bottom-right of the status bar shows one button "M @me:example.org ▾" (avatar, label, chevron). Card titled with the account: Settings, Voice & audio, Sessions, a divider, a filled red/pink Log out, then Close. No pick_list is built for the account (remaining pick_list is the room notification menu); the card is plain buttons, so wheel has nothing to pick.

## Log out sends Message::LogoutRequested; other entries send the old messages; card closes after a choice.
Status: PROVEN
```console
test screens::main_view::tests::the_account_card_reaches_every_message_the_old_bar_sent ... ok
test state::tests::sidebar_menu_flow::log_out_in_the_account_card_asks_through_the_logout_flow ... ok
test state::tests::sidebar_menu_flow::the_account_card_opens_closes_with_esc_and_closes_after_each_choice ... ok
```
Code: `AccountAction::LogOut => Message::LogoutRequested` (main_view.rs:1076); the state test asserts the card closes and the logout flow (warning or Command::Logout) runs.

## Esc closes the card right after the room switcher.
Status: PROVEN (state-level test; no real keyboard)
```console
test state::tests::sidebar_menu_flow::the_account_card_opens_closes_with_esc_and_closes_after_each_choice ... ok
```
The test opens the card, then the switcher over it: first Esc closes only the switcher, second Esc closes the card. Esc alone also closes it.

## No Log out button outside the card.
Status: PROVEN
```console
$ grep -rn '"Log out"' crates/app/src
main_view.rs:1087 (card entry label), main_view.rs:1250 (card debug test list)
```
Only other "Sign out anyway" is the logout warning in sessions.rs. The screenshot's status bar holds only the sync state and the account button.

## Verdict
EVIDENCE: PROVEN
