# The sidebar

How to arrange the left side of Twicecat (the spaces rail and the room list of
a space), where that arrangement is kept, and how to start over. For
keyboard use see [accessibility.md](accessibility.md), section "The sidebar".

## What you can arrange

**The spaces rail**

- Put your spaces in the order you like.
- Group spaces into **folders**. A closed folder shows a 2 x 2 preview of its
  spaces and the total of their unread counts; an open folder lists them. A
  folder can have a name (1 to 32 characters) or none.
- Move a space into a folder, out of it, or into a new folder. Ungroup a folder
  to give its spaces back to the rail. A folder left with one space stays until
  you ungroup it; moving a space out of a folder with two dissolves it.
- A short line under **Home** parts it from your spaces.
- A space that has **sub-spaces** (the spaces it lists as its own children;
  deeper levels are not shown) shows a small arrow at the bottom left of its
  icon when you point at it, and always while its sub-spaces are shown. Click
  the arrow, or use **Show sub-spaces** and **Hide sub-spaces** in the space's
  menu, to list them under the space, in a tinted band, in the order the server
  gives. Click a sub-space to show its rooms: the room list then shows that
  sub-space as a space of its own (Channels, Direct messages and your own
  sections for it). A closed space whose sub-space is selected looks selected.
  Sub-spaces cannot be dragged, moved into a folder or given a menu. Which
  spaces are shown open is **not saved**: after a restart or a logout every
  space is closed again.

**Filter chips** above the room list pick what the list shows, one at a time:
All, Unreads, Mentions, DMs, Rooms, Favourites, Low priority, Invites and
Spaces. Click a chip to filter; click the chosen chip again to go back to All.
The chips go onto more lines when the pane is narrow, and they are not drawn
when the room list is collapsed to its narrow rail. A chip other than All shows
the flat list (the sections show only under All, with no search). Unreads lists
the rooms with unread messages or marked unread; Mentions lists the rooms the
server counts as having a mention or unread messages, so a room you marked as
unread can show there too. Favourites and Low priority list the rooms carrying
those Matrix tags, so a room favourited in another Matrix client such as
Element shows under Favourites. Each chip has its own line when nothing
matches, such as "No unread rooms".

**Sort** is a button at the right of the search field (it is not drawn when the
room list is collapsed to its narrow rail). It opens a card with
two choices: **Order**, which is Activity (the newest activity first; the
default) or A to Z (by the room's name, ignoring upper and lower case, and then
by its ID), and **Unreads first** (off by default), which puts the rooms with a
mention first, then those with unread notifications, then those with unread
messages or marked as unread, and keeps the order inside each group. A change
applies at once, is kept on this device in the settings file (not in the
sidebar file), and is still there after a restart. It applies to the flat list
and to every section whose order you did not make: Starred and the sections of
your own keep the order you gave them, whatever the sort. In the flat list with
the All chip, rooms you marked as low priority come after all the others, in
the chosen order. With A to Z or Unreads first Twicecat loads every page of
your rooms, so that a room from a later page lands in its place; a very long
list can take a moment to settle. Alt+Up and Alt+Down, and Ctrl+Shift+Up and
Ctrl+Shift+Down, walk the rooms in the order shown. No keyboard shortcut was
added for the sort.

**The room list of a space**, drawn as sections you can collapse and expand:
Starred, Favourites, Channels (the space's own rooms), one section per
sub-space, the sections of your own, Direct messages and Low priority. A
collapsed header shows the unread or mention count of its rooms.

The list is drawn like Discord's categories. A header is the section's name in
capitals, small and muted, with its arrow on the left; the name turns to the
normal text colour under the pointer, and the "⋯" button stays. A room is a
**channel row**: the room's picture before its name, or its first letter when
it has none (as in the flat list), and a small speaker at the bottom right
corner of the picture on a voice room. A direct message has its picture too
and keeps its usual row. A
channel row has no message preview and no time, and it is as tall as before in
both densities. A room with unread messages, notifications or a mark has its
name in bold in the normal text colour, with a small bar at the left edge of
the list; a room you have read has its name in the muted colour; and a room set
to Mute has its name dimmed even when it has unread messages, as Discord
dims it, and keeps its Muted label and its badge. The badges are the same as
in the flat list (a mention, a count, or a dot), so nothing is hidden. The
sort's "Unreads first" still counts a muted room with unread messages as
unread, as the badge does. Home, a search and the filter chips keep the rows
with a picture, a preview and a time.

- **Star** a room to put it under Starred. Unstar returns it to its default
  section.
- Make **sections of your own**: create, rename and delete them, and move rooms
  into them. Deleting a section sends its rooms back to their default sections.
- **Favourites** and **Low priority** are Matrix room tags (`m.favourite` and
  `m.lowpriority`). Unlike the rest of the arrangement they are **kept on the
  homeserver**, so other Matrix apps such as Element show them too, and a room
  you tagged there appears here. Use **Add to favourites**, **Remove from
  favourites**, **Mark as low priority** and **Remove low priority** in a
  room's menu; the words follow the room's tags, and a room has at most one of
  the two. Each section shows only while it has rooms, and can be collapsed and
  moved like the others. A room goes in the first section that applies:
  Starred, Favourites, one of your own, Low priority, its sub-space, Direct
  messages, Channels. Where the two sections stand and whether they are
  collapsed is still kept on this device only. If a tag cannot be changed, a
  line above the composer says so. You can also drag a room onto these two
  headers (see "You can also drag" below).
- **Tags can be changed from the flat list too.** Right-click a room in Home, in
  the list under a chip other than All, or in a search result, and its card
  offers the same two tag entries, with the same words, and nothing else: a
  move, Star and sections need a space's sectioned list. A tag button changes
  the tag, closes the card and saves no sidebar file; under the Favourites chip
  "Remove from favourites" makes the room leave that list when the server
  reports it. The narrow rail (a room list collapsed to icons) has no card: widen
  the list to tag a room.
- Reorder sections, and reorder rooms inside a section of your own.
- A room can go into Starred or a section of your own, and back to its own
  default section. It cannot be moved from one built-in section to another.

The rooms and spaces themselves are not changed: this is only how your copy
of Twicecat shows them. Nobody else sees your folders and sections, and the
server is not told about them. The one exception is the Favourites and Low
priority tags above.

**Voice after text.** In every section whose order you did not choose (Favourites,
Channels, a sub-space, Direct messages and Low priority), text rooms come first
and voice rooms after them, each group in the chosen sort. Starred and your own
sections keep exactly the order you gave them, a voice room first if you put it
there.

**A collapsed category keeps the open room and your call.** Collapsing a
section hides its rooms, with two exceptions: the room that is open stays drawn
under the header, and so does the voice room of the call this device is in,
with the people listed under it. Every other room is hidden, an unread one
too, and the header's count still counts all the rooms of the section, the kept
ones included. A section with neither room in it shows its header alone.
Alt+Up and Alt+Down walk the rooms that are drawn, so they skip the hidden
rooms and stop at a kept one.

**Voice channels** list the people in the call under the room (up to eight,
then "and N more"). The speaking ring and the muted icon are shown only for the
call this device is in.

## How to arrange it

Everything can be done through the **options menus**:

- **right-click** a space, a folder, a room or a section header, or
- press the **"⋯"** button at the right of a section header (or on the open
  room's row).

The menu is a card in the middle of the window with the entries Move up, Move
down, Move to folder (which also offers a new folder), Remove from folder,
Rename, Ungroup, Show sub-spaces and Hide sub-spaces (on a space that has
sub-spaces), Move to section (which also offers New section), Back to its
default section, Star, Unstar, Add to favourites, Remove from favourites, Mark
as low priority, Remove low priority, Collapse, Expand and Delete section.

You can also **drag**, in two places:

- **On the rail**, a drag only **reorders**: press a space or a closed folder
  and move it up or down, and the others slide aside to make room. The
  spaces inside an open folder are reordered inside that folder, and an open
  folder itself is moved with its menu. Making a folder, putting a space into
  a folder and taking a space out of one are done **only** with the menu
  (Move to folder, which also offers a new folder, and Remove from folder).
  A space whose sub-spaces are shown is not dragged, by its icon or its band,
  and neither are its sub-spaces or its arrow: move it with its menu (Move up,
  Move down), or hide its sub-spaces first. Home cannot be dragged.
  An open folder and a space whose sub-spaces are shown are not dragged: they
  move with their own menu. A finger on a touch screen does not reorder the
  rail either; the menu makes the same move.
  Esc, leaving the window or moving the pointer out of the rail ends the drag
  and changes nothing.
- **In the room list**, a room onto a section header or between the rows of
  one of your own sections, and a section header onto another header. Dropping
  a room onto the Favourites or Low priority header gets that tag, as "Add to
  favourites" or "Mark as low priority" do, and it is kept on the homeserver
  (no sidebar file is saved). Dropping a room onto its own section (the one it
  would be in with no tag, such as Channels, a sub-space or Direct messages)
  removes the tag it carries, as "Remove from favourites" or "Remove low
  priority" do. A room already in Favourites dropped on Favourites does
  nothing, and the same goes for Low priority. A starred room is not allowed on
  Low priority, and the target is drawn as not allowed: Starred comes first, so
  use Unstar first. A room that has a tag and that you drag into one of your
  own sections keeps its tag: the move is saved, but a favourite stays under
  Favourites (the tag wins), so remove the tag to see it in your section.
  A row of Channels, a sub-space, Direct messages, Favourites, Low priority
  or Starred takes a dropped room like its header does: dropping a room on a
  row of a built-in section works like dropping it on that section's header
  (the whole row shows the same ring, or "Not allowed"), and the sort, not
  the drop, decides where it stands. Between the rows of a section of your own
  the drop puts the room before or after a row, and the gap between two rows
  is one place: one line is drawn there, and it does not move while the pointer
  crosses from the lower half of one row to the upper half of the next.

  Some drops have a result you cannot see at once:

  - A favourite that you moved into a section of your own stays under
    Favourites, as said above, until you remove the tag.
  - A starred room dropped on Favourites gets the tag and stays under Starred,
    where it was: Starred comes first, so nothing changes on the screen. The
    tag is there, and Element shows it.
  - A favourite that is also saved in a section of your own, dropped on
    Channels, removes the tag and the room reappears in that section, which is
    what Remove from favourites does too.
  - A room that is the only one in its own section leaves that header hidden
    when it is tagged, because a section shows only while it has rooms, so
    there is nothing to drop on: use Remove from favourites or Remove low
    priority in its card.

Every drop does the same thing as the menu entry of that move; nothing can be
done only by dragging. A drag does not scroll the list, so use the menu for a
target that is out of view. Esc, or leaving the window, cancels a drag.

No keyboard shortcut was added for any of this. The menus open with a pointer
(a right-click or a click), so someone who uses only the keyboard sees the
arrangement but cannot change it. Tagging is done with a pointer only, too, in
a space's list and in the flat list. The tags are private to the account, not
public: every user of Twicecat on that account sees them in the Favourites and
Low priority sections, keyboard-only users included, whether they were set
here or in another Matrix app, and every room is still found with Ctrl+K.

## Where it is kept: on this device, and the two tags on the homeserver

The tags are kept on the homeserver: Favourites and Low priority are the
Matrix tags of the room, so Element and other clients show them, and a tag set
there shows here. The layout and the sort are kept on this device: the layout
in the sidebar file below, the sort in `settings.json`. Which spaces are shown
open on the rail is not saved at all.

The arrangement (the order, the folders, the sections and the names you gave
them) is kept **on this device only**. Only the two tags above reach your
homeserver; the arrangement is not sent to it and it is not shared with your
other devices or other Matrix clients: a second computer starts with the
default sidebar and has its own arrangement.

There is **one file per account**, in the same folder as `settings.json`,
named `sidebar-<digest>.json`, where `<digest>` is 16 characters (0 to 9 and
a to f) worked out from the account's user ID. The file holds the account's
user ID, the order, folders and sections, room and space IDs and the names you
gave. It has no message, no room name and no token.

| System | Folder |
|---|---|
| Linux | `$XDG_CONFIG_HOME/twicecat`, or `~/.config/twicecat` when that variable is not set |
| macOS | `~/Library/Application Support/io.github.ga_lep.Twicecat` |
| Windows | `%APPDATA%\ga_lep\Twicecat\config` |

With `--data-dir <dir>` the files are in `<dir>`. With `--ephemeral` the
folder is new and empty each time, so nothing is kept from one run to the next.

- **The file is kept when you log out** and is read again the next time the
  same account logs in on this device, so your arrangement is still there. A
  log out forgets it in memory only; the file stays on disk.
- **Another account on the same device never reads or changes your file.**
  Each account uses its own file, and the account's ID is also written inside
  it and checked, so a file cannot be mistaken for another account's.
- If a file cannot be read at all (it is damaged), it is set aside as
  `sidebar-<digest>.json.bad` and the default sidebar is shown.

## Starting over

- **Reset sidebar layout** in **Settings, Appearance**: press **Reset sidebar
  layout...**, then confirm with **Reset sidebar layout** in the question (or
  **Cancel**). It removes the file of the account you are logged in as, and
  the rail and the room lists go back to the default. It does not touch the
  file of any other account.
- Or quit Twicecat and **delete the file** by hand.

## Limits

The arrangement has caps so that the file stays small. A change that would
pass one is not made.

- Up to 200 entries on the rail (spaces and folders together), at most 50 of
  them folders.
- Up to 20 sections of your own in one space, and up to 500 rooms in its
  Starred and own sections together.
- Folder and section names are 1 to 32 characters, with no control or hidden
  characters.
- Up to 4,000 room and space references in the whole file, and a file larger
  than 256 KiB is not read.

## "Made by a newer Twicecat" and "belongs to another account"

An older Twicecat does not know the section keys for Favourites and Low
priority, which a newer one writes: it drops the favourites and lowpriority
keys it does not know when it reads the file, and loses only where you had
placed or collapsed those two sections. This is not "made by a newer Twicecat"
(below), because nothing else is lost.

Two kinds of file are **left exactly as they are**. While one is in the way,
the sidebar shows the default arrangement, it cannot be changed, and
**Reset sidebar layout...** is off, with a line that says why on the
Appearance page.

- **Made by a newer Twicecat.** The file says it was written by a later version
  of the program than the one you run now (for example after you went back to
  an older version). This version does not know its format, so it does not
  read it and it never writes over it. Run the newer Twicecat again to use it,
  or delete the file by hand to start over.
- **Belongs to another account.** The file at your account's name says it was
  made for a different account (a file that was copied or renamed by hand, or a
  rare clash of names). It is not yours to read or change, so it is left alone.
  Delete it by hand to start over.
