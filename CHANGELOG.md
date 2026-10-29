# Changelog

All notable changes to Desktop Companion are documented here.
The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- **📊 System monitor widget**, pinned on the desktop like the calendar and the
  clock: CPU with a one-minute graph, RAM, free space on the Windows drive,
  download and upload speed, battery (laptops only) and how long the PC has
  been on. It refreshes every 2 seconds and measures nothing while it is not
  pinned. Bars turn gold from 70% and red from 90%; Miu cheers you on when the
  PC is busy, looks startled when it is overloaded and dozes when it is idle.
  Pin it from Settings → 🧩 Widget, 💞 → 📊 on the ring menu or the tray, and
  🧹 Tidy lines it up with the other widgets. Skins and decorations from the
  Collection apply to it too.
- **See what is slowing the PC down.** Click CPU or RAM on the system monitor
  and it lists the five apps using the most, grouped like Task Manager does
  (all of Chrome's processes on one line, with a ×23 count). Switch between
  CPU and RAM, open Task Manager with one click, or ↩ go back. The app list is
  read only while this panel is open and only shown there: it is not saved,
  not sent anywhere and the pet does not use it. Nothing is ever closed for
  you.
- **The pet warns you about a low battery and a full drive** (Settings →
  awareness, on by default). Below 15% without the charger she asks you to
  plug in, and again more urgently at 5%; each level once per drain. When the
  Windows drive has less than 10% free she says how much is left, at most once
  a day. Like deadlines, these reach you as a Windows notification when she is
  hidden, and they may interrupt a focus session.

## [0.10.0] - 2026-10-29

### Added
- **🗒️ Notes, for the things you look up every day.** A separate board from
  Tasks, with no due dates and nothing to tick off: Wi-Fi details, phone
  numbers, links, commands you type often. Open it from the tray, the pet's
  right-click list or 💞 Together in the ring menu.
  - Search as you type, with or without Vietnamese accents ("mat khau" finds
    "Mật khẩu").
  - ⭐ keeps a note at the top; the rest show the most recently edited first.
  - 📋 copies a note's content in one click; links open in the browser.
  - 📌 pins a note to the desktop as a yellow sticky note, held by the same
    pushpin as pinned tasks, with its own 📋 Copy button. Drag it anywhere,
    📍 keeps it above other windows, ✕ takes it off the desktop.
  - Select text in any app and pick 🗒️ **Save as note** on the flower: the
    Notes board opens with a new note already holding that text, ready for a
    title.
  - Edit in place with ✏️ or a double-click; Ctrl+Enter saves, Esc cancels.
    Ctrl+N starts a new note and Ctrl+F jumps to the search box.
  - Long notes fold to eight lines; click to read the rest.
  - Notes are kept in their own `notes.json`. If that file is ever damaged, the
    board says so and refuses to overwrite it instead of starting over empty.
    Every save also keeps the previous version as `notes.json.bak`, and the
    error message points to it.
- **🎗 Events on the pinned calendar, separate from tasks.** Memorials,
  birthdays and anniversaries are no longer tasks: they never go overdue, do not
  appear in the Tasks board and have nothing to tick off.
  - **＋ Add** now asks whether it is a ✓ Task or a 🎗 Event. An event happens
    once, every year on the same date (↻ Yearly), or every year on the same
    **lunar** date (↻ Lunar), for death anniversaries and lunar birthdays.
  - Lunar repeats follow the usual custom: a leap month does not repeat the
    event, and a 30th falls back to the 29th in a month with only 29 days.
    A yearly event on 29 February falls on the 28th in other years.
  - The pet still reminds you: the chosen days before, on the morning, an hour
    before and at the time, and again from the start next year. When she
    cannot speak, a Windows notification goes out as for deadlines. On the
    day itself her morning greeting names the event too.
  - The 🎗 **Events** tab of the Notes board lists every event by how soon it
    comes next ("Today", "Tomorrow", "In 9 days"), with its time, repeat and
    lunar date. Click one to open the calendar on that day; past one-off
    events sit at the bottom.
  - Events show as a gold dot on the day and are listed first in the day panel,
    with ↻ for repeating ones. The day's list now scrolls instead of cutting off
    at two items.
  - Click an event or a task to edit it, switch it between task and event, or
    delete it (🗑, then confirm). Tasks you added from the calendar before stay
    tasks until you switch them.
  - Events are kept in their own `events.json`, protected against damage and
    backed up to `events.json.bak` like notes.

## [0.9.11] - 2026-10-28

### Added
- **🎁 A free Candy Capsule spin every day.** The first time you are at the
  computer each day, a small capsule sits on top of the character's head and
  follows her when she moves or changes size. Click it and the capsule shakes,
  then opens into a gift card with a bow, a glowing medal, stars for the rarity
  and a burst of sparkles (a rainbow frame for a Legendary model). It is a
  normal spin in every way except the price: it counts towards the Rare and Epic
  guarantees, shows in the spin history and refunds candy for a duplicate.
  🍬 View opens the Collection right at the item you won; × hides the capsule
  until the next time the app starts, and the spin stays yours until midnight.
- **Three more Live2D models in the Collection**, cat girls with smile, sad,
  angry, surprised, shy and wink expressions and their own motions, downloaded
  once unlocked:
  - **🌸 Pink Aristocrat** (10000 🍬): a pink-haired noble with long curls, in a
    pink gothic lolita dress with black lace, bows and hearts.
  - **🐰 Pink Kawaii** (5000 🍬): pink hair, an oversized white bunny jacket, a
    pink plaid skirt and bunny sneakers.
  - **❄️ Snow Kawaii** (5000 🍬): blue-tinted silver hair and glasses, a white
    jacket with blue bows, a frilled skirt and bunny boots.

### Changed
- **Updated Gothic Aristocrat and Snow Aristocrat models.** Installed copies
  update themselves at the next launch, with no app update needed.
- **The Legendary tier stays at 3%, now split twelve ways.** Each 10000 🍬 model
  is 0.1%, each 7500 🍬 model 0.2% and each 5000 🍬 model 0.4%, so the cheaper
  a model, the easier she is to win. Common, Rare and Epic are unchanged at
  62 / 25 / 10.

### Fixed
- **🔇 Mute is remembered.** Muting the pet from the Sound menu now lasts across
  restarts and model changes; before, she started talking and playing sounds
  again every time the app opened. While muted she no longer prepares spoken
  replies at all.

## [0.9.10] - 2026-10-23

### Added
- **The Pastel trio joins the Collection at 7500 🍬 each**, cat girls in glasses
  and white frills with smile, sad, angry, surprised, shy and wink expressions
  and their own motions, downloaded once unlocked:
  - **🦇 Gothic Pastel**: lilac-silver hair, black and purple bows, plush bunnies.
  - **🌸 Pink Pastel**: pastel-pink hair, pink bows and cherry blossoms.
  - **❄️ Snow Pastel**: silver hair, fur sleeves and blue snowflake bows.

### Changed
- **Updated Gothic Aristocrat, Pink Angel and Snow Aristocrat models**, with
  Pink Angel moving to a sharper 2048 px texture. Installed copies update
  themselves at the next launch, with no app update needed.
- **The Legendary tier stays at 3%, now split nine ways.** Each 10000 🍬 model
  is 0.2%, each 7500 🍬 model 0.3% and each 5000 🍬 model (Snow Lolita, Gothic
  Lolita, Snow Dancer) 0.5%, down from 0.8%.

## [0.9.9] - 2026-10-22

### Added
- **Three more Live2D models in the Collection**, each with smile, sad, angry,
  surprised, shy and wink expressions and her own motions, downloaded once
  unlocked:
  - **❄️ Snow Aristocrat** (10000 🍬): a silver-haired cat-eared noble with long
    curls, in a white and blue frilled dress of snowflakes and ice crystals.
  - **🦇 Gothic Lolita** (5000 🍬): a lavender-haired cat girl in glasses and a
    black and purple gothic lolita dress of hearts and plush bunnies.
  - **❄️ Snow Dancer** (5000 🍬): a street dancer with blue-tipped brown hair, a
    snowflake cap, a white jacket and a chained black skirt.

### Changed
- **The Legendary tier stays at 3%, now split six ways.** Each 10000 🍬 model
  (Gothic Aristocrat, Pink Angel, Snow Aristocrat) is 0.2%; each 5000 🍬 model
  (Snow Lolita, Gothic Lolita, Snow Dancer) is 0.8%. Common, Rare and Epic are
  unchanged at 62 / 25 / 10.

## [0.9.8] - 2026-10-21

### Added
- **🎀 Widget decor in the Collection.** A new Decor tab dresses the calendar,
  clock and weather widgets together: ribbon bows, washi tape and sakura with
  falling petals (Common), lace trim, a twinkling star garland and a snowcap with
  falling flakes (Rare), and a shifting rainbow holo frame (Epic). Trade candy
  for them or win them in the Candy Capsule; the decor never blocks clicks and
  keeps clear of the widget buttons.
- **Early reminders for calendar events.** When you add something on the pinned
  calendar, pick when to hear about it ahead of time: 1 day, 3 days, 1 week,
  10 days or 1 month before (any mix; 1 day is on by default). The pet tells you
  at 7 am that day, or Windows shows a notification if she can't, and the usual
  same-day reminders still follow. Events with early reminders show a 🔔.
- **Edit events on the calendar.** Click an event to change its name, time or
  early reminders right in the calendar. Renaming keeps its reminder history;
  changing the time starts the reminders over. "+n more" opens the Tasks panel.

### Changed
- **The three widgets match.** Calendar, clock and weather now share one width,
  the same padding and the same faded Miu, so "Tidy up" lines them up on both
  edges. The weather tint is lighter, so the card keeps your skin's colour.

## [0.9.7] - 2026-10-21

### Added
- **❄️ Snow Lolita**, a third Live2D model in the Collection: a silver-haired
  cat girl in glasses and a white fur coat of snowflakes and ice crystals, with
  smile, sad, angry, surprised, shy and wink expressions and her own idle, nod,
  tilt, shake and happy motions. She costs 5000 🍬, half the other models, and
  downloads once unlocked.

### Changed
- **Candy Capsule rates are now 62 / 25 / 10 / 3.** Snow Lolita has her own 2%
  Legendary rate (cheaper model, easier to win); Gothic Aristocrat and Pink
  Angel still share 1%. The extra comes out of the Common share, so Rare and
  Epic stay the same.

## [0.9.6] - 2026-10-21

### Added
- **🎭 Pink Angel**, a second Live2D model in the Collection: a pink-haired
  cat girl in glasses and a frilly dress of hearts and plush bunnies, with
  smile, sad, angry, surprised, shy and wink expressions and her own idle, nod,
  tilt, shake and happy motions. Trade 10000 🍬 for her or win her in the Candy
  Capsule; like Gothic Aristocrat she downloads once unlocked.

### Changed
- **The Legendary rate stays at 1%, now shared by both models.** Each Legendary
  pull is Gothic Aristocrat or Pink Angel with equal odds (0.5% each); the
  Candy Capsule rates are still 64 / 25 / 10 / 1.

## [0.9.5] - 2026-10-20

### Changed
- **Artwork license.** The original artwork (the Gothic Aristocrat Live2D model,
  model previews, side characters, Miu faces and Arcade art) is now licensed
  under CC BY-NC 4.0: free to share and remix with credit, not for commercial
  use. The code stays MIT. See ASSETS-LICENSE.md.

### Fixed
- **Selection flower.** The icon chosen in the Collection (paw, star, clover,
  snowflake) now shows every time you select text, with its own colour. Before,
  the flower switched back to 🌸 each time it appeared.
- **Saving Settings no longer undoes a model change.** If you switched the
  character from the Appearance menu or the Collection while Settings was open,
  pressing Save put the old model back the next time the app started.
- **Candy Capsule drop rates fit their panel again.** With the Legendary row
  added, the pity counters and the refund note were cut off. Each Legendary
  model's rate now sits on the Legendary row (hover for the refund), and the
  panel scrolls if it ever runs out of room.

## [0.9.4] - 2026-10-20

### Added
- **🎭 Live2D models in the Collection.** A new Models tab. The first one is
  **Gothic Aristocrat**, a gothic young noble in black lace with smile, sad,
  angry, surprised, shy and wink expressions and her own idle, nod, tilt,
  shake and happy motions. Trade 10000 🍬 for her or win her in the Candy
  Capsule, then press Use and she replaces the character on screen right away.
  Hiyori, Haru, Mao and Miara stay free, and so do models you add from a folder.
- **Collection models download on unlock.** They are not in the installer: once
  you unlock one, the app downloads it (about 3 MB, progress shown on its card)
  and checks it against a signed list before installing. Models update on their
  own when a newer version is published, without an app update. If the
  download fails, the card says why and offers to try again.
- **🌈 Legendary rarity**, above Epic and only for Live2D models. Each model has
  its own drop rate (Gothic Aristocrat 1%), taken from the Common share, so Rare
  stays 25% and Epic 10% (Common is now 64%). The rates panel lists every model
  with its rate and refund. A Legendary spin gets its own reveal: rainbow rays,
  a longer shake, heavy confetti and a colour-shifting "LEGENDARY!".

### Changed
- The Appearance menu marks locked models with 🔒. Choosing one opens the
  Collection on the Models tab instead of switching.
- The 50-spin guarantee is now Epic or better, so it can also land a Legendary.
- Duplicates refund 10% of the item's own price (1000 🍬 for Gothic
  Aristocrat). Common, Rare and Epic refunds are unchanged.

### Fixed
- Removing a model you added from a folder and then adding another no longer
  gives two models the same id, which made one of them show the other's model.

## [0.9.3] - 2026-10-19

### Added
- **🎰 Candy Capsule.** A capsule machine in the Collection window: 100 🍬 for
  one spin or 1000 🍬 for ten, across every item in the Collection. Drop rates
  are shown right on the machine (Common 65%, Rare 25%, Epic 10%). The 10th
  spin without a Rare or better is always Rare or better, and the 50th without
  an Epic is always Epic. Duplicates refund 10% of the item's price. Each spin
  shakes the bunny machine and drops a capsule; it then bursts open in the
  colour of the best item: a pink pop for Common, violet rays and stars for
  Rare, and a white flash, golden rays, falling confetti and a glowing
  "EPIC!" for Epic, with the prize popping out of the capsule. Click to skip.
  The window now has three parts: Candy Capsule, the Candy shop (trade
  directly, as before) and a History of your last 50 spins. Still free, still
  never sold for real money.
- **Four more widget skins:** Matcha, Cotton candy, Café latte and Neon night.
- **Three more reward effects:** a heart shower on headpats and friendship
  level-ups, soap bubbles with the morning greeting, and a shooting star
  (instead of sakura petals) when you finish a task after 10 pm.
- **💬 Speech bubbles.** A new Collection group that changes how the pet's
  speech bubble looks: Cloud, Heart, Gothic lace and Scroll.
- **🌸 Selection flower.** A new Collection group that swaps the 🌸 shown when
  you select text for a ⭐, 🍀, 🐾 or ❄️.
- **🦊 Fox shrine panels.** With the Hồ ly character set, Settings, Tasks, the
  text assistant and every other window turn crimson with cream cards, gold
  borders and sleeping foxes, lanterns and torii gates in the corners, like the
  existing Snow and Gothic looks.

### Changed
- **New candy scale.** Everything earns 10× more candy (a milestone is now
  +200 🍬), and shop prices are 10× higher (600 / 1500 / 3000 🍬), so
  spinning is the cheap road and trading is the sure one. Candy you already
  had is multiplied by 10 once, so it buys exactly what it bought before.

## [0.9.2] - 2026-10-19

### Added
- **🍬 Candy and a Collection.** Everything that already earns friendship
  points (finished tasks, focus sessions, chat, headpats, game wins, being
  around) now also earns candy, with the same daily caps, and every friendship
  milestone adds 20 more. Spend it in the new **🍬 Collection** window — open
  it from 💞 → 🍬 on the ring, the 🎁 tile on the Mini Games page, or Settings →
  *🎨 Diện mạo*. Candy is only for decorations: nothing useful is ever locked,
  and nothing is sold for real money.
- **Widget skins.** Five new colour themes (Sakura, Night sky, Mint, Sunset,
  Ocean) for the pinned calendar, clock and weather widgets, applied to all
  three at once the moment you pick one. The weather widget still tints with
  the sky (rain, night, sunshine) on top of any skin.
- **Reward effects.** Fireworks when you finish every task of the day, a
  sakura shower each time you finish a task, and confetti when a focus session
  ends; turn each one on or off after unlocking,
  and preview them before you buy.
- **❄️ Snow Lounge**, a new character set for Bunny Lounge (Epic, 300 🍬): an
  ice-crystal cat girl, a snow bunny and a snow fox lean on Settings, Tasks,
  the text assistant, the weather card and Mini Games. While it is in use,
  the Settings panel turns icy blue: each group becomes a frozen glass card with
  snow on top, snow falls gently, see-through ice-and-snow corners (a snow
  bunny, a snowman and a snowy cottage) frame the panel, and a Christmas tree
  stands behind the pet.
- **🐱 Neko Lounge**, a new character set (Epic, 300 🍬): the gothic purple cat
  girl from the old Bunny Lounge and a new punk cat girl in a cat-ear hoodie. While
  it is in use, the Settings panel turns gothic lolita: dark glass cards with
  purple trim, drifting violet sparkles and see-through amethyst corners (a
  black bunny, a snowman and a gothic cottage).
- **🦊 Fox Lounge**, a new character set (Epic, 300 🍬): a shrine-maiden fox
  with golden bells and the steampunk fox girl from the old Bunny Lounge.
- **🦇 Gothic Miu**, a new face set for Miu (Rare, 150 🍬): a cat-eared Miu
  in a purple-and-black gothic lolita dress with nine expressions.
- **❄️ Snow Miu**, a new face set for Miu (Rare, 150 🍬): a cat-eared Miu in
  a white knit coat with nine winter expressions. The chosen face set shows on
  the pinned widgets, the Mini Games page and inside every game.
- Miu cheers when you unlock something, and the 💗 friendship card shows how
  much candy you have.

### Changed
- Every panel follows the character set in use: Settings, Tasks, the text
  assistant, the weather card, the Collection, the Mini Games page and all three
  games turn icy blue with Snow Lounge and gothic lolita with Neko Lounge, with
  matching corner decorations, and switch the moment you change sets.
- Bunny Lounge is now just its two bunny girls (pink and lilac), taking turns
  on the five panels; the cat girls moved to Neko Lounge and the steampunk fox
  to Fox Lounge.
- Bunny Lounge characters and the mini Miu faces on the widgets can now be
  swapped while the app runs, ready for new character and face sets.

### Fixed
- Pinning a widget from Settings, the ring or the tray brings it to the front,
  so a widget with 📍 off no longer opens hidden behind the window you are
  using (it looked as if the calendar would not open).
- The "LIVE2D" label under the pet is readable on light backgrounds.

## [0.9.1] - 2026-10-16

### Added
- **The pet talks about the calendar.** On a holiday she wishes you a happy
  day by its name, with her own lines for Tết and Lunar New Year's Eve, and
  she mentions the full moon (*Rằm*) and the 1st of each lunar month. The
  evening before a day off, the Mid-Autumn Festival, a *Rằm* or a 1st, she
  reminds you it is tomorrow, so there is time to buy flowers and fruit for
  the offering. Each line is said once (it is not repeated when you restart
  the app), only in the daytime for today's day and in the evening for
  tomorrow's, and like every other line she stays quiet while you focus, in
  Do not disturb or full screen. Turn it off in Settings → *🧩 Widget*.
- **A clock you can pin to your screen, with world clocks.** It shows your
  PC's time with the weekday and lunar date, plus up to two more clocks for
  other places (Tokyo, Seoul, New York, London…), each with the time
  difference and "tomorrow" / "yesterday" when the date differs; summer time
  is handled automatically. The 🍅 row starts or stops a focus session (the pet
  joins in, as from the ring) and counts down while it runs.
- **🧩 Widget in Settings** replaces the *📅 Lịch* group: tick the month
  calendar, the clock or the weather widget to pin it right away, pick the
  extra clocks, and **🧹 tidy** every pinned widget into one column on the
  right of the main screen.
- **Your birthday on the calendar.** The day you entered in Settings gets a
  🎂, and clicking it shows a birthday line with Miu cheering.

### Changed
- **Four new faces for Miu.** Sleepy, surprised, shy and "you can do it!"
  now show up where they fit: she cheers you on during a focus session and
  on days with tasks due, gets sleepy on the clock late at night (11 pm–5 am)
  and on the weather widget at night, is startled by overdue tasks, a
  thunderstorm and an exploding bomb in Bomb Squad, and looks sheepish when
  the weather can't be fetched or a game of guess the number runs out of
  tries. On a clear day the weather card has her cheering.

## [0.9.0] - 2026-10-16

### Added
- **A month calendar you can pin to your screen.** Turn it on from the tray
  (*📅 Ghim / gỡ lịch tháng*), 💞 → 📅 on the right-click ring, or the new
  *📅 Lịch* group in Settings. Every day
  shows its Vietnamese lunar date (the 1st of each lunar month in gold, the
  15th as *Rằm*), with Vietnamese holidays such as Tết, Hùng Kings Day,
  Mid-Autumn and New Year's Eve marked, and days off in red. Days with tasks
  due get a dot, red when something is overdue. Click a day to see its full
  lunar date, its holiday and what is due, or to add a task due that day at
  the time you pick. A mini Miu sits in the corner, cheering on holidays and
  looking puzzled when something is overdue. Flip months with ‹ › or the mouse wheel. Today moves on
  by itself after midnight. It stays above other windows by default; 📍
  turns that off, ✕ unpins it, and it remembers where you dragged it. Shown
  in Vietnamese, Japanese or English, following the message language.

### Changed
- New artwork for the girls on the edge of the weather card and the Mini
  Games page (Bunny Lounge).

### Fixed
- The pet no longer disappears after you unplug a second monitor: if the
  place she was saved at is off every screen, she comes back to the bottom
  corner of the main screen.
- "Start with Windows" opens the installed app again. The installed app now
  updates the startup entry to point at itself each time it runs.

## [0.8.0] - 2026-10-16

### Added
- **Weather effects across your whole screen.** Turn them on in Settings →
  *🌦 Thời tiết*: pick rain, snow, cherry blossoms, falling leaves, fog,
  sunshine or a starry night (light, medium or heavy); or choose **"By season
  & time"** — blossoms in spring, sunshine in summer, leaves in autumn, snow
  in winter, stars at night, no internet needed; or **"Real weather"**, which
  follows the sky in the city you picked, including thunderstorms with
  optional soft lightning. Rain splashes and snow settles on top of the
  taskbar. The effect never blocks your mouse or keyboard, stays under the pet,
  leaves the app's own panels clear, and pauses by itself when you watch
  something full screen, lock the PC, step away or run on battery (optional).
  Every monitor gets its own layer. Switch it on or off with **Ctrl+Alt+W**
  (changeable), the tray, or ☁️ → 🌦 on the ring. Off by default.
- **A weather forecast card.** Open it from the tray (*☁️ Dự báo thời tiết*),
  the ☁️ group on the right-click ring, or Settings. It shows the sky and
  temperature right now, today's high and low, seven days ahead and a 24-hour
  chart with the chance of rain, in °C or °F, plus a line from Miu. Type your
  city's name right inside the card (click the city name to change it later).
  Your location is never looked up: only the coordinates of the city you
  choose are sent to [Open-Meteo](https://open-meteo.com) (free, no key
  needed), every 30 minutes and only while you use the weather features. When
  the connection drops it keeps the last data, marked as not updated.
- **Pin the weather to your desktop**, like a pinned task: a small widget with
  Miu in the corner, the temperature, today's high / low and the chance of
  rain in the next hour. Make it larger to see the next three days, keep it on
  top with 📍, drag it anywhere — it remembers where it was. Double-click opens
  the full forecast. Pin or unpin it from the card or ☁️ → 📌 on the ring.
- **The pet notices the weather.** When the effect changes (or you just turned
  it on) she says one line about it in your language and pronouns — "It's
  raining~ take an umbrella ☔", "So many stars tonight…" — with a matching
  face. At most one line per kind of weather every two hours, never just
  because you opened the app, and it can be turned off. With a city chosen she
  also gives a short **morning report** once a day: today's low and high and
  the chance of rain, with an umbrella reminder when rain is likely.

### Changed
- **The right-click ring has a ☁️ Weather group** (forecast, effects on / off,
  pin to desktop) where 🔕 used to be; 🔕 *Do not disturb* moved into
  💞 *Đồng hành*. Settings stays in the bottom slot.
- **🌸 Bunny Lounge reaches two more panels:** a steampunk fox girl leans on
  the weather card and a gothic cat girl hugging a purple plush cat on the
  Mini Games page (when Bunny Lounge is on).

## [0.7.0] - 2026-10-16

### Added
- **A Mini Games page.** 💞 → 🎮 now opens a page of its own instead of the
  small card on the pet: three illustrated cards (Rock paper scissors, Guess
  my number, Bomb Squad) with your best result on each and a *Play now*
  button, a "★ Today" game of the day, your friendship level and how many
  points games gave you today, plus Achievements (plays, wins and records for
  every game) and Rewards.
- **New mini game: Bomb Squad (minesweeper).** Pick it on the Mini Games page.
  Choose Easy (8 × 8, 10 bombs), Normal (12 × 12, 25) or Hard (16 × 16, 40);
  left-click opens a tile, right-click plants a flag, and clicking a number
  whose bombs are all flagged opens the rest around it. Your first tile is
  always safe. The top bar shows the timer, bombs left and flags placed. Miu
  sits on the right and reacts as you play: nervous while you hold a tile,
  happy when it is safe, startled by a bomb and cheering when the board is
  clear. Her tip box explains the rules and can light up a safe tile
  (3 hints on Easy, 2 on Normal, 1 on Hard). ⚙ turns sound, quick-open and
  her chatter on or off and shows your plays, win rate and fastest time per
  level. Closing the window mid-game pauses the clock and keeps the board.

### Changed
- **⚙ Settings sits at the bottom of the round menu** (it swapped places with
  🗂️ *Mở đầy đủ*), so it is easy to find.
- **Rock paper scissors has its own big window now.** A playful card with the
  game's title, a You vs. pet scoreboard, the character on the right with a
  speech bubble, and three large cards to pick from (or keys Q · W · E). Your
  pick lights up while she thinks, then the round shows both hands side by
  side with a big *Win! / Lose… / Draw!*. A framed portrait above the cards
  shows Miu reacting to the game: a wink while she waits, a puzzled "?" while
  she picks, cheering when she wins, a pout when she loses and a big smile on
  a draw. ⚙ opens game settings: play to 3, 5 or 7 wins, choose the character
  (Miu, the cat girl or either bunny), difficulty (easy lets you win more,
  hard learns which hand you like), sound effects, animations, and how many
  friendship points games have given you today.
- **Guess the number has its own window too.** Miu sits on the left and
  reacts to every guess (puzzled when it is too low, pouting when it is too
  high, cheering when you get it). Type with the on-screen keypad or your
  keyboard, then *Guess* (Enter), *Clear* or ask for a *Hint*. The right panel
  shows the range, hearts for the guesses you have left and a history of every
  try. ⚙ lets you pick the range (1–10, 20, 50 or 100), 5, 7 or 10 guesses,
  whether to show "hot / cold" hints, sound, and difficulty (easy gives 3
  hints, normal 1, hard none), plus your plays, win rate and best streak.

## [0.6.0] - 2026-10-15

### Added
- **Focus sessions (Pomodoro).** Pick 🍅 (right-click → 💞 *Đồng hành*) or use
  the tray: 25 minutes of focus, then a 5-minute break (both adjustable in
  Settings → *Tập trung*). A ring on the pet counts down, the pet wears its
  focused face, and nothing interrupts you except a deadline. When the session
  ends she cheers you on and sends you to rest; pick ⏹ to stop early.
- **The pet notices when you step away.** After 10 minutes without mouse or
  keyboard (or with the screen locked) she dozes off, without a word. When you
  come back she wakes up and greets you — differently after a short break, a
  long absence, or the next morning.
- **Optional CPU / RAM warning** (off by default). When CPU or memory stays
  above your threshold for 2 minutes, the pet tells you, at most once every 30
  minutes for each. With it off, nothing is measured at all.
- **A bond that grows over time.** Being at the computer each day, finishing
  tasks and focus sessions, chatting and head-pats slowly raise your bond with
  your companion, through five levels from *Mới quen* to *Không thể thiếu*.
  Each kind of activity counts only up to a small amount per day, and nothing
  is ever taken away when you skip a day. Pick 💗 (right-click → 💞) to
  see the level, progress to the next one, days together and your streak. The
  closer you are, the more casually she talks in chat.
- **💞 Đồng hành** on the right-click ring gathers 🍅 focus, 💗 bond and 🎮
  games, so the first ring stays tidy.
- **Mini games.** Right-click → 💞 *Đồng hành* → 🎮 *Chơi*: rock paper scissors
  (first to 3 wins) or guess the number (1–100 in 7 tries, with higher /
  lower hints). The pet reacts to every round with her face and shows her
  move (✊ ✌️ ✋), your latest guess, and finally her secret number in a
  bubble above her head. Games take a minute or two, and each finished game
  adds a little to your bond.
- **🗂️ Mở đầy đủ on the right-click ring** opens the text assistant next to the
  pet with an empty box to type or paste your own text — translate, reply or
  save it as a task without selecting anything first.
- **A hello to start the day**, with how many tasks are due today.
- **Milestones and birthdays.** She celebrates 7, 30, 100 and 365 days
  together, each yearly anniversary, every new bond level, 7- and 30-day
  streaks — each one once — and your birthday, if you add it in Settings →
  *Bạn đồng hành* (day and month only).
- Settings → *Nhận biết máy tính*: doze-off delay (or off), greet on return,
  stay quiet while presenting or full screen, and the CPU / RAM warning. The
  app reads only on/off states and percentages — never window titles, app
  names or screen content.

## [0.5.0] - 2026-10-15

### Added
- **Pin a task to your desktop.** Press 📌 next to a task in the Tasks panel
  and it becomes a pink sticky note on the desktop, held by a pushpin with
  your companion's face on it. Drag it anywhere; it stays where you left it,
  also after a restart. Hover the note for its buttons: **✓ Xong** stamps it
  done, drops it off the screen and the pet cheers you on; **✕** unpins it
  (the task stays in your list); **📍** keeps it above other windows. Double-
  click a note to open the Tasks panel. Up to 8 notes at a time.
- **Companion girls cling to the panels.** A pink-haired bunny idol leans on
  the left edge of the Settings window, a silver-haired bunny on Tasks, and a
  blue-haired cat girl on the text assistant (*Mở đầy đủ* from the flower),
  about a third of each overlapping the frame. The overlapping part
  fades to a faint ghost so the panel stays readable, and clicks go through
  to the panel underneath. They follow the panel when you
  move or resize it, hide with it, and never take clicks. If a panel opens too
  close to the left side of the screen, it slides right just enough for her
  to fit. Turn them off (or back on) with Settings → *Diện mạo* → *🌸 Bunny
  Lounge* (off by default); it applies right away.
- **Links in tasks are clickable.** A web link in a task or its note — on a
  pinned note or in the Tasks panel — opens in your default browser. Only
  `http://` and `https://` links open; anything else stays plain text.
- **Give your companion a name.** Until she has one, she asks you now and then
  (at most three times, a week apart) with a small card right on the pet:
  type a name, or pick *Để sau*. She thanks you by name, then asks how she
  should address you. You can also name her in chat ("từ giờ em tên là Mây
  nhé"); the reply notes the new name only once it is really saved. Change it
  any time in Settings → *Bạn đồng hành*.
- **Choose how she addresses you**: em – anh, em – chị, mình – bạn, tớ – cậu,
  em – sếp, or your own words. When she speaks Japanese or English she uses
  your name instead (with さん / くん / ちゃん / 様 in Japanese).
- **Deadlines for tasks.** Press 📅 on a task to give it a date and time. The
  Tasks panel sorts by the nearest deadline and shows how urgent each one is
  ("⏰ Còn 20 phút", "⚠️ Quá hạn 1 ngày"); pinned notes show it too. The pet
  reminds you once per step: on the morning of the due day, an hour before,
  when it is due, and once a day for up to three days after. Tell the pet "nhắc
  anh gửi báo cáo trước 5 giờ chiều mai" and the task is saved with its
  deadline.
- **Windows notifications for deadlines.** If a deadline arrives while the pet
  is hidden, in a panel, set to do-not-disturb, or you are presenting full
  screen, you get a Windows notification instead (Settings → *Nhắc nhở*, with
  a *Gửi thử* button).
- **Rest reminders, on by default**: rest your eyes every 45 minutes, drink
  water and stand up every 60 minutes of sitting at the computer. Stepping
  away for 5 minutes counts as a rest. Reminders that fall close together are
  combined into one line. They need no API key.
- **The pet cheers when you finish a task** — from the Tasks panel, a pinned
  note, or chat — with something special for the last task on the list,
  finishing before the deadline, and your 3rd, 5th and 10th task of the day.
- **Finish or delete tasks by chat.** Say you are done with a task and the pet
  ticks it off. It only changes a task when it is sure which one you meant,
  and says so when it is not.
- **Do not disturb.** Pick 🔕 on the pet's right-click ring or in the tray
  menu and the pet stops starting conversations for an hour. Pick it again
  (🔔) to turn it back off early. Your questions in chat are still answered.

### Changed
- **The text assistant (*Mở đầy đủ* from the flower) now matches the Settings
  look**: the same soft pink background, a sakura glass card, pink section
  headings, pill tabs and the same inputs and buttons.
- **The pet no longer talks to an empty room.** It waits instead of starting a
  conversation while you are away from the computer (5 minutes without mouse
  or keyboard, or the screen is locked), while a game, video or slideshow is
  full screen, or while it is hidden. Only these on/off states are read, never
  window titles, app names or screen content.
- Lines the pet says on its own are now at least 10 minutes apart (adjustable
  in Settings → *Nhắc nhở*). Deadline reminders do not wait.
- **"Ngôn ngữ chữ" now sticks and reaches the chat.** The language picked in
  the pet's sound menu (or in Settings) is saved, survives a restart, and the
  pet's lines and chat replies follow it in Vietnamese, Japanese or English.
  Menus and panels are still in Vietnamese.

### Fixed
- **Minimizing Settings or Tasks no longer leaves the pet stranded.** The pet
  that stands in the middle of the panel stayed floating where the panel had
  been. It now goes back to its usual spot when the panel is minimized, and
  steps back into the panel when you open it again.

## [0.4.1] - 2026-10-14

### Changed
- **A new app icon** — a pink-haired, cat-eared girl with headphones and a
  lilac laptop, the same art as the VS Code extension's new icon. Every size
  in `src-tauri/icons/` (Windows `.ico`, macOS `.icns`, the PNG sizes, the
  Store logos, Android and iOS) is regenerated with `tauri icon` from one
  1024 × 1024 transparent source; the tray icon reads `icons/icon.png`, so it
  changes too.
- **New fallback art in the pet window** — `app/media/icon.png` and
  `app/media/character.png` (shown while no Live2D model is loaded, and as the
  chat avatar before a chibi is captured) match the new look.

### Fixed
- **A chat reply that never arrives is now really cancelled** — after 30
  seconds without a first word, the bubble still says no reply came, and the
  request is now also stopped in the app, so a late answer no longer slips
  into the chat history behind your back.

## [0.4.0] - 2026-10-13

### Added
- **The pet changes its face while it talks.** The model tags its reply with
  emotions (`[happy]`, `[shy]`, `[surprised]`…); the tags are hidden from the
  bubble and switch the Live2D expression as the reply streams in.
- **Quick Chat remembers the conversation** (last 10 turns), also across
  restarts. It is saved only on this machine, and the 🗑 button in the chat
  panel clears it.
- **"Hỏi em" on the selection flower.** Hold the 🌸 and pick 💭 to ask the pet
  about the text you just selected; the panel shows the attached snippet.
- **The pet knows the time and your unfinished tasks**, so "hôm nay còn việc gì
  không em?" gets a real answer.
- **The pet starts the conversation now and then.** After a quiet stretch
  (Settings → *Em tự bắt chuyện*: off / 30 min / 1 h / 2 h, default 1 h) it says
  one line: nudges a task that has been sitting longest, or reminds you to rest
  your eyes. It stays quiet while hidden, docked in a panel, or mid-reply.
- **Ask the pet to remember a task.** Say "nhắc anh gửi báo cáo cho sếp" and it
  adds the task to your list. The bubble confirms with "📝 Đã lưu task: …" only
  once the task is really saved, or says why it could not be saved. Nothing is
  saved if you stop the reply halfway.
- **A ⏹ button on the bubble stops the pet mid-reply.**

### Changed
- **Quick Chat is one click away.** The right-click menu used to open a "Chat AI"
  ring that held nothing but Quick Chat; **💬 Chat nhanh** now sits directly on
  the first ring.

### Fixed
- **"Hỏi em" works with click-through on.** The chat panel used to ignore your
  clicks and typing. Click-through is now paused while you ask, and turned back
  on once the pet has answered. If the pet is sitting in the Settings or Tasks
  panel, it steps out first so the chat panel has room.
- **Your Gemini API key no longer shows in error messages.** Translate, Quick
  Reply and the model list put the key in the request address, and a
  connection error printed that address on screen. The key now travels in a
  header.
- **Saving Settings no longer forgets "Bỏ qua bản này"**, so a skipped update
  stays skipped.
- **Quick Chat (right-click → Chat AI) now answers.** The question was sent but
  nothing handled it, so the bubble sat on "Em đang suy nghĩ…" until it timed
  out. Replies now stream in word by word from Anthropic or Gemini.

## [0.3.2] - 2026-09-15

### Added
- **An update dialog that opens itself on launch.** The app already checked for
  a new version ~5s after start, but it only nudged you with a pet bubble saying
  to go find the button in Settings — so unless you went looking, you never
  updated. It now opens a centred dialog with the new version number, the one
  you are on, the release notes, and a **Tải & cài ngay** button that downloads
  and installs right there.
- **A progress bar while the update downloads.** The download reports real
  percentages (falling back to an indeterminate sweep when the server sends no
  content length) instead of one line of text and then silence until the app
  restarts itself.
- **"Bỏ qua bản này".** The automatic dialog stays quiet about that exact
  version on later launches; checking manually from Settings clears the skip.

### Changed
- **Kiểm tra cập nhật** in Settings now opens that same dialog — release notes
  and progress bar included — instead of installing from within the panel.

## [0.3.1] - 2026-09-14

### Added
- **The pet steps into the Settings and Tasks windows.** Opening either panel
  moves the live Live2D pet into the gap left between their two columns and
  makes it click-through, so it can't be grabbed or poked while you work. Close
  the panel and the pet returns to its exact previous position and size, with
  interaction restored. Dragging or resizing the panel carries the pet along.
- **Tasks: separate "to do" and "done" lists.** Completing a task moves it to
  the ✅ column, most recently finished first; each column shows a count.
- **Tasks: timestamps.** Every task shows when it was created (`🕒 Tạo`) and,
  once finished, when it was completed (`✓ Xong`). Same-day times show only the
  clock; older ones include the date. Un-checking a task clears its completion
  time and returns it to the to-do column.
- **Tasks: editable notes.** Click a task's note line to edit it in place —
  Enter saves, Shift+Enter adds a line, Escape cancels.
- **Model and Motion pickers as two side columns**, ported from the VS Code
  companion. The pickers used to be one card centred over the character, so
  listing a dozen motions hid the very model you were dressing. They now sit in
  two narrow columns either side, leaving the middle clear, and stay open across
  choices — only × or Escape closes them.

### Changed
- **Settings is a two-column panel.** Provider / API key / model and the system
  options sit on the left, translation and appearance on the right, with the pet
  between them. The window grew to 880×600 and collapses to two even columns
  below 620px wide.
- **The quick-translate window is smaller and auto-sizing.** It now shows only
  the translation by default and grows or shrinks to fit the text instead of
  leaving a fixed empty box. The two "copy" buttons became 📋 icons inside each
  box, and a 👁 icon reveals the original text — which, when shown, is labelled
  so the two boxes can be told apart.

### Fixed
- **The Motion list reflects the model.** It was three hard-coded names
  (`TapBody` / `TapHead` / `Idle`), so a model naming its groups `待机` or `摸头`
  showed three buttons that did nothing. The list is now read from the loaded
  model, with a note when a model declares no motions at all.
- **Docking the pet no longer loses its position.** The window-moved handler
  saves the pet's position on every move, which would have overwritten the
  remembered spot with the temporary one inside a panel; it now skips saving
  while the pet is docked, and quitting from the tray un-docks first.

## [0.3.0] - 2026-09-12

### Added
- **Radial menu on the selection flower.** Press-and-hold the 🌸 now fans the
  actions out on a ring around the cursor instead of opening the 3-tab panel:
  🌐 translate · 📋 copy · 💬 reply · 📝 save as task · 🗂️ full panel. Each entry
  goes straight to the window it needs — picking **Reply** or **Save as task**
  lands you on that tab, rather than always on Translate.
- **Radial right-click menu on the pet**, ported from the VS Code companion.
  Hovering an icon shows its name in the centre pill; clicking a category swaps
  the ring for that category's entries and turns the pill into a back button.
  The classic list menu is still reachable by setting `window.__MENU_STYLE__`
  to `'list'`.

### Changed
- **The 📋 icon above the flower is gone.** Copying now lives on the radial menu
  (📋), and **Ctrl+C** while the flower is on screen still works exactly as
  before. The flower window shrank to a single 30×30 button, so the flower lands
  directly on the cursor instead of being offset upward to make room for the icon.

### Fixed
- **The full panel honours the tab it was asked for.** `popup-refresh` carried a
  tab name from Rust but the popup ignored it and always reset to Translate.

## [0.2.3] - 2026-07-01

### Fixed
- **The selection flower no longer steals keyboard focus.** When you drag-select
  text, the 🌸 flower now pops up _without_ pulling focus away from the app you
  selected in — your caret and the highlighted text stay exactly where they were,
  so you can keep typing or editing right away. Previously the (pre-created,
  reused) flower window called `set_focus()` on every show, which activated the
  overlay and moved focus off the highlighted paragraph. The flower stays fully
  clickable thanks to its `WS_EX_NOACTIVATE` style and remains on top via
  `always_on_top`.

## [0.2.2] - 2026-06-29

### Added
- **Copy the original selected text.** A 📋 icon now appears above the flower as
  soon as text is captured. Click it — or press **Ctrl+C** while the flower is on
  screen — to put the captured text on the clipboard. This recovers the selection
  that the auto-capture would otherwise discard (the capture restores your
  previous clipboard, and the on-screen highlight is often cleared when the flower
  appears).
- **"Copy original text" button** in both the quick-translate window and the full
  3-tab panel, next to the existing "copy translation" button.

### Fixed
- **Saving Settings no longer shows _"The system cannot find the file specified.
  (os error 2)"_** on a fresh install. The "Start with Windows" (autostart) toggle
  used to unconditionally delete a registry entry that does not exist yet on a new
  machine; it now only updates the entry when needed.

### Notes
- The Ctrl+C copy shortcut is registered **only while the flower is visible** and
  is released the moment it disappears, so it never permanently hijacks the system
  Ctrl+C. While the flower is on screen (a few seconds), Ctrl+C copies the captured
  text instead of the focused app's selection.

## [0.2.1] - 2026-06-28

### Added
- **Signed auto-updates.** The app checks GitHub Releases for a newer signed
  version on launch and offers to update; Settings also has a manual
  "Kiểm tra cập nhật / Check for updates" button.

## [0.2.0] - 2026-06-27

### Added
- Initial public release: Live2D desktop pet with select-to-translate, quick
  reply, save-task, translation contexts, look-at-cursor, `Ctrl`+wheel resize,
  custom Live2D models, interaction sounds, Anthropic Claude / Google Gemini
  providers, and a customizable capture hotkey.
- Landing page published to GitHub Pages.

[0.10.0]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.10.0
[0.9.11]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.9.11
[0.9.10]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.9.10
[0.9.9]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.9.9
[0.9.8]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.9.8
[0.9.7]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.9.7
[0.9.6]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.9.6
[0.9.5]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.9.5
[0.9.4]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.9.4
[0.9.3]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.9.3
[0.9.2]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.9.2
[0.9.1]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.9.1
[0.9.0]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.9.0
[0.8.0]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.8.0
[0.7.0]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.7.0
[0.6.0]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.6.0
[0.5.0]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.5.0
[0.4.1]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.4.1
[0.4.0]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.4.0
[0.3.2]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.3.2
[0.3.1]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.3.1
[0.3.0]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.3.0
[0.2.3]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.2.3
[0.2.2]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.2.2
[0.2.1]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.2.1
[0.2.0]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.2.0
