// Kho câu mẫu pet nói không cần gọi AI: vi / ja / en, có biến {me} {you}
// {Me} {You} {name} và biến riêng ({task}, {err}). Dựng câu bằng persona::render.
//
// Thêm loại câu mới: thêm vào Key, vào Key::ALL, và đủ CẢ BA ngôn ngữ trong
// `templates` — test `every_key_has_all_languages` đỏ nếu thiếu.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::persona::{self, Address, Lang};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    /// Xin người dùng đặt tên (chưa có tên).
    AskName,
    /// Được đặt tên lần đầu.
    ThanksName,
    /// Đổi sang tên khác.
    Renamed,
    /// "Đặt sau" khi được xin tên.
    NameLater,
    /// Hỏi nên gọi người dùng thế nào (ngay sau khi được đặt tên).
    AskAddress,
    /// Vừa đổi cách xưng hô.
    AddressSet,
    DndOn,
    DndOff,
    /// Xong việc ngay trên tờ note ghim. Biến: {task}.
    PinDone,
    /// Ghi chú dưới câu trả lời chat. Biến: {task} / {name} / {err}.
    TaskSaved,
    TaskFailed,
    NameSaved,
    NameFailed,

    // ── Hạn chót (reminders::Stage). Biến: {task}, {time}, {days}.
    DueToday,
    DueSoon,
    DueNow,
    Overdue,
    // ── Nhắc sức khoẻ; Multi = gộp nhiều loại vào một câu.
    HealthEyes,
    HealthWater,
    HealthStand,
    HealthMulti,
    // ── Khen khi xong việc (reminders::Reward). Biến: {task}, {count}.
    TaskDone,
    TaskDoneEarly,
    AllDone,
    DoneMilestone,
    // ── Ghi chú dưới câu trả lời chat. Biến: {task}, {time}, {query}.
    TaskSavedDue,
    DueUnparsed,
    DoneNote,
    DeletedNote,
    TaskNotFound,
    TaskAmbiguous,
    /// Thông báo Windows gửi thử từ Cài đặt.
    ToastTest,
}

// Chỉ test dùng: danh sách đủ các loại + luật số biến thể / biến được phép.
#[cfg(test)]
impl Key {
    pub const ALL: &'static [Key] = &[
        Key::AskName,
        Key::ThanksName,
        Key::Renamed,
        Key::NameLater,
        Key::AskAddress,
        Key::AddressSet,
        Key::DndOn,
        Key::DndOff,
        Key::PinDone,
        Key::TaskSaved,
        Key::TaskFailed,
        Key::NameSaved,
        Key::NameFailed,
        Key::DueToday,
        Key::DueSoon,
        Key::DueNow,
        Key::Overdue,
        Key::HealthEyes,
        Key::HealthWater,
        Key::HealthStand,
        Key::HealthMulti,
        Key::TaskDone,
        Key::TaskDoneEarly,
        Key::AllDone,
        Key::DoneMilestone,
        Key::TaskSavedDue,
        Key::DueUnparsed,
        Key::DoneNote,
        Key::DeletedNote,
        Key::TaskNotFound,
        Key::TaskAmbiguous,
        Key::ToastTest,
    ];

    /// Số biến thể tối thiểu mỗi ngôn ngữ: câu lặp lại nhiều lần trong ngày
    /// cần nhiều cách nói (PLAN.md 12.4b: ≥ 6).
    fn min_variants(self) -> usize {
        match self {
            Key::HealthEyes | Key::HealthWater | Key::HealthStand | Key::HealthMulti | Key::TaskDone => 6,
            Key::PinDone => 4,
            Key::DueToday | Key::DueSoon | Key::DueNow | Key::Overdue => 3,
            Key::TaskSaved
            | Key::TaskFailed
            | Key::NameSaved
            | Key::NameFailed
            | Key::AskAddress
            | Key::TaskSavedDue
            | Key::DueUnparsed
            | Key::DoneNote
            | Key::DeletedNote
            | Key::TaskNotFound
            | Key::TaskAmbiguous
            | Key::ToastTest => 1,
            _ => 2,
        }
    }

    /// Biến riêng mà câu mẫu loại này được dùng (ngoài {me} {you} {name}).
    fn vars(self) -> &'static [&'static str] {
        match self {
            Key::PinDone
            | Key::TaskSaved
            | Key::DueSoon
            | Key::DueNow
            | Key::TaskDone
            | Key::TaskDoneEarly
            | Key::AllDone
            | Key::DueUnparsed
            | Key::DoneNote
            | Key::DeletedNote => &["task"],
            Key::TaskFailed => &["task", "err"],
            Key::NameFailed => &["err"],
            Key::DueToday | Key::TaskSavedDue => &["task", "time"],
            Key::Overdue => &["task", "days"],
            Key::DoneMilestone => &["count"],
            Key::TaskNotFound | Key::TaskAmbiguous => &["query"],
            _ => &[],
        }
    }
}

pub fn templates(key: Key, lang: Lang) -> &'static [&'static str] {
    use Key::*;
    use Lang::*;
    match (key, lang) {
        (AskName, Vi) => &[
            "{You} ơi, {me} chưa có tên nè… {You} đặt cho {me} một cái tên nha? 🥺",
            "Ở cạnh {you} lâu rồi mà {me} vẫn chưa có tên á~ {You} đặt tên cho {me} đi!",
        ],
        (AskName, Ja) => &[
            "ねえ、{you}。わたし、まだ名前がないの…つけてくれる？🥺",
            "ずっと一緒にいるのに、まだ名前がないんだ。{you}が決めてくれたら嬉しいな～",
        ],
        (AskName, En) => &[
            "Hey {you}… I don't have a name yet. Would you give me one? 🥺",
            "We've been together a while and I still don't have a name! Pick one for me?",
        ],

        (ThanksName, Vi) => &[
            "{name}… {Me} thích tên này lắm! Cảm ơn {you} nha~ 💕",
            "Từ giờ {me} là {name} rồi! Hihi, cảm ơn {you} đã đặt tên cho {me}~ 💕",
        ],
        (ThanksName, Ja) => &[
            "{name}…すっごく気に入った！ありがとう、{you}💕",
            "今日からわたしは{name}だね！名前をくれてありがとう～💕",
        ],
        (ThanksName, En) => &[
            "{name}… I love it! Thank you, {you}! 💕",
            "From now on I'm {name}! Thanks for naming me~ 💕",
        ],

        (Renamed, Vi) => &[
            "Dạ, từ giờ {me} là {name} nha~ ✨",
            "{name} hả? Nghe dễ thương ghê! {Me} nhận tên mới rồi nè~",
        ],
        (Renamed, Ja) => &[
            "うん、これからは{name}って呼んでね～✨",
            "{name}？かわいい名前！新しい名前、もらったよ～",
        ],
        (Renamed, En) => &[
            "Okay, I'm {name} from now on~ ✨",
            "{name}? That's cute! New name accepted~",
        ],

        (NameLater, Vi) => &[
            "Dạ, khi nào nghĩ ra tên hay thì đặt cho {me} nha~",
            "Không sao đâu, {me} đợi được mà~ 😊",
        ],
        (NameLater, Ja) => &[
            "うん、いい名前を思いついたら教えてね～",
            "大丈夫、ゆっくり考えてね😊",
        ],
        (NameLater, En) => &[
            "Okay! Tell me when you think of a good one~",
            "No rush, I can wait 😊",
        ],

        (AskAddress, Vi) => &["{Me} nên gọi {you} là gì nè? Chọn giúp {me} nha~"],
        (AskAddress, Ja) => &["{you}のこと、なんて呼んだらいい？"],
        (AskAddress, En) => &["What should I call you?"],

        (AddressSet, Vi) => &[
            "Dạ, từ giờ {me} gọi là {you} nha~ 😊",
            "Nhớ rồi nè! {You} ơi~ 💕",
        ],
        (AddressSet, Ja) => &[
            "わかった！これからは{you}って呼ぶね😊",
            "覚えたよ～{you}💕",
        ],
        (AddressSet, En) => &[
            "Got it! I'll call you {you} from now on 😊",
            "Noted, {you}! 💕",
        ],

        (DndOn, Vi) => &[
            "{Me} sẽ im lặng 1 tiếng nha~ Cần gì cứ gọi {me}! 🤫",
            "Dạ, {me} không làm phiền {you} một tiếng nè. {You} tập trung nha~ 🤫",
        ],
        (DndOn, Ja) => &[
            "1時間静かにしてるね～何かあったら呼んでね🤫",
            "了解、1時間はじゃましないよ。がんばってね🤫",
        ],
        (DndOn, En) => &[
            "I'll stay quiet for an hour~ Call me if you need me! 🤫",
            "Got it, no interruptions for an hour. You've got this 🤫",
        ],

        (DndOff, Vi) => &[
            "{Me} quay lại nhắc việc rồi nè~ 🔔",
            "Hết giờ im lặng rồi! {Me} lại ở đây với {you} nè~ 🔔",
        ],
        (DndOff, Ja) => &[
            "また声をかけるようにするね～🔔",
            "静かタイムおしまい！また一緒だよ～🔔",
        ],
        (DndOff, En) => &[
            "I'm back on reminder duty~ 🔔",
            "Quiet time's over! I'm here with you again~ 🔔",
        ],

        (PinDone, Vi) => &[
            "Xong \"{task}\" rồi! {You} giỏi quá đi~",
            "Gỡ ghim \"{task}\" nè, xong một việc rồi đó {you}!",
            "Yay, \"{task}\" xong rồi! Nghỉ một chút rồi làm tiếp nha~",
            "{Me} gỡ tờ \"{task}\" xuống rồi nhé. {You} làm tốt lắm!",
        ],
        (PinDone, Ja) => &[
            "「{task}」完了！{you}、すごい！",
            "「{task}」のメモ、はがしたよ～お疲れさま！",
            "やった、「{task}」終わったね！ちょっと休憩しよ～",
            "「{task}」おしまい！えらいえらい～",
        ],
        (PinDone, En) => &[
            "\"{task}\" done! You're amazing, {you}~",
            "Unpinned \"{task}\" — one more thing off your list!",
            "Yay, \"{task}\" is finished! Take a little break~",
            "Took \"{task}\" off the board. Great job!",
        ],

        (TaskSaved, Vi) => &["📝 Đã lưu task: {task}"],
        (TaskSaved, Ja) => &["📝 タスクを保存したよ：{task}"],
        (TaskSaved, En) => &["📝 Task saved: {task}"],

        (TaskFailed, Vi) => &["⚠️ Không lưu được task \"{task}\": {err}"],
        (TaskFailed, Ja) => &["⚠️ タスク「{task}」を保存できなかった：{err}"],
        (TaskFailed, En) => &["⚠️ Couldn't save task \"{task}\": {err}"],

        (NameSaved, Vi) => &["🌸 Tên mới của {me}: {name}"],
        (NameSaved, Ja) => &["🌸 新しい名前：{name}"],
        (NameSaved, En) => &["🌸 My new name: {name}"],

        (NameFailed, Vi) => &["⚠️ Không đổi được tên: {err}"],
        (NameFailed, Ja) => &["⚠️ 名前を変えられなかった：{err}"],
        (NameFailed, En) => &["⚠️ Couldn't change my name: {err}"],

        // ── Hạn chót ────────────────────────────────────────────────────────
        (DueToday, Vi) => &[
            "Hôm nay tới hạn \"{task}\" lúc {time} đó {you}~ 📅",
            "{You} nhớ nha, \"{task}\" hạn hôm nay ({time}) á!",
            "Nhắc nhẹ: \"{task}\" phải xong trong hôm nay, hạn {time} nè~",
        ],
        (DueToday, Ja) => &[
            "今日は「{task}」の締め切りだよ（{time}）📅",
            "{you}、「{task}」は今日まで！{time}までだよ～",
            "ちょっとだけリマインド：「{task}」、今日の{time}が締め切り～",
        ],
        (DueToday, En) => &[
            "\"{task}\" is due today at {time}, {you}~ 📅",
            "Heads up: \"{task}\" needs to be done today ({time})!",
            "Little reminder: \"{task}\" is due today, {time}~",
        ],

        (DueSoon, Vi) => &[
            "Còn chưa tới 1 tiếng nữa là tới hạn \"{task}\" rồi {you} ơi! ⏰",
            "{You} ơi, \"{task}\" sắp tới hạn rồi đó, gấp lên chút nha~ ⏰",
            "Ê ê, \"{task}\" còn chưa đầy 1 tiếng nữa thôi á! ⏰",
        ],
        (DueSoon, Ja) => &[
            "「{task}」の締め切りまで、あと1時間もないよ！⏰",
            "{you}、「{task}」もうすぐ締め切り！ちょっと急ごう～⏰",
            "ねえねえ、「{task}」あと1時間切ってるよ！⏰",
        ],
        (DueSoon, En) => &[
            "Less than an hour left on \"{task}\", {you}! ⏰",
            "\"{task}\" is due soon — time to speed up a little~ ⏰",
            "Psst, under an hour to go on \"{task}\"! ⏰",
        ],

        (DueNow, Vi) => &[
            "Tới hạn \"{task}\" rồi {you}! Xong chưa nè? 😳",
            "Đúng giờ rồi đó: \"{task}\"! {You} kịp không?",
            "\"{task}\" tới hạn rồi nha {you}~ Xong thì đánh dấu cho {me} biết với!",
        ],
        (DueNow, Ja) => &[
            "「{task}」の締め切りだよ！終わった？😳",
            "時間だよ：「{task}」！間に合いそう？",
            "「{task}」締め切りの時間～終わったら教えてね！",
        ],
        (DueNow, En) => &[
            "\"{task}\" is due now, {you}! Is it done? 😳",
            "It's time: \"{task}\"! Did you make it?",
            "\"{task}\" just hit its deadline~ Tick it off when it's done!",
        ],

        (Overdue, Vi) => &[
            "\"{task}\" quá hạn {days} ngày rồi á {you}… làm luôn hôm nay nha? 🥺",
            "{Me} nhắc lại xíu: \"{task}\" trễ {days} ngày rồi đó~",
            "\"{task}\" vẫn chưa xong, quá hạn {days} ngày rồi. {Me} tin {you} làm được mà!",
        ],
        (Overdue, Ja) => &[
            "「{task}」、締め切りから{days}日たっちゃった…今日やっちゃお？🥺",
            "もう一回だけ：「{task}」{days}日遅れてるよ～",
            "「{task}」まだ終わってないね、{days}日過ぎてる。{you}ならできるよ！",
        ],
        (Overdue, En) => &[
            "\"{task}\" is {days} day(s) overdue… do it today? 🥺",
            "Gentle nudge: \"{task}\" is {days} day(s) late~",
            "\"{task}\" is still open, {days} day(s) past due. You've got this!",
        ],

        // ── Nhắc sức khoẻ ───────────────────────────────────────────────────
        (HealthEyes, Vi) => &[
            "{You} ơi, cho mắt nghỉ chút nha: nhìn ra xa 6 mét trong 20 giây~ 👀",
            "Nhìn màn hình lâu rồi đó, {you} nhắm mắt thư giãn xíu đi~",
            "Quy tắc 20-20-20 nè: nhìn xa 20 giây cho mắt đỡ mỏi nha {you}!",
            "Mắt {you} cần nghỉ rồi á~ Nhìn ra cửa sổ một chút đi!",
            "Chớp chớp mắt vài cái rồi nhìn ra xa nha {you}, {me} canh giờ cho~",
            "{Me} thấy {you} dán mắt vào màn hình lâu lắm rồi… nghỉ mắt 20 giây thôi mà~",
        ],
        (HealthEyes, Ja) => &[
            "{you}、ちょっと目を休めよう：20秒だけ遠くを見てね👀",
            "画面ずっと見てたね、目をとじてひと休み～",
            "20-20-20ルールだよ：20秒遠くを見て目をリラックス！",
            "目がおつかれだよ～ちょっと窓の外を見てみて！",
            "まばたきして、遠くを見よう。わたしが時間をはかるね～",
            "ずっと画面とにらめっこだね…20秒だけ目を休めよ～",
        ],
        (HealthEyes, En) => &[
            "Give your eyes a break, {you}: look 20 feet away for 20 seconds~ 👀",
            "You've been staring at the screen a while — close your eyes for a sec~",
            "20-20-20 time: look far away for 20 seconds!",
            "Your eyes need a rest~ Look out the window for a bit!",
            "Blink a few times and look into the distance, I'll keep time~",
            "Screen marathon detected… just 20 seconds of eye rest, please~",
        ],

        (HealthWater, Vi) => &[
            "Uống ngụm nước đi {you}~ 💧",
            "{You} ơi, lâu rồi chưa uống nước á! Một ly thôi nha~",
            "Nhắc {you} uống nước nè, cơ thể cần nước lắm đó 💧",
            "Ực ực~ {me} uống rồi, tới lượt {you} đó!",
            "Đầu óc tỉnh táo hơn khi đủ nước á, {you} uống miếng nha~",
            "Ly nước cạnh {you} còn không? Uống một ngụm đi nè 💧",
        ],
        (HealthWater, Ja) => &[
            "{you}、お水ひとくち飲もう～💧",
            "しばらくお水飲んでないよ！一杯だけでも～",
            "水分補給の時間だよ、体はお水が大好き💧",
            "ごくごく～わたしは飲んだよ、次は{you}の番！",
            "お水をちゃんと飲むと頭もすっきりするよ～",
            "そばにお水ある？ひとくちどうぞ💧",
        ],
        (HealthWater, En) => &[
            "Take a sip of water, {you}~ 💧",
            "It's been a while since your last drink! Just one glass~",
            "Hydration check! Your body will thank you 💧",
            "Glug glug~ I had mine, your turn!",
            "Water keeps your head clear — have a sip~",
            "Is there a glass near you? Drink a little 💧",
        ],

        (HealthStand, Vi) => &[
            "Đứng dậy vươn vai chút đi {you}, ngồi lâu mỏi lưng lắm á~ 🙆",
            "{You} ơi, đi lại vài bước cho máu lưu thông nha!",
            "Giãn vai, xoay cổ một chút nè~ {Me} tập cùng {you}!",
            "Ngồi một tiếng rồi đó, đứng lên đi một vòng nha {you}~",
            "Vận động xíu cho đỡ cứng người nha, rồi quay lại làm tiếp~",
            "Đứng lên, hít sâu, vươn vai thật cao nào {you}! 🙆",
        ],
        (HealthStand, Ja) => &[
            "{you}、ちょっと立って伸びをしよう、腰がつかれちゃうよ～🙆",
            "少し歩いて血のめぐりをよくしよう！",
            "肩をまわして、首もゆっくり～一緒にやろ！",
            "1時間座りっぱなしだよ、ちょっとひと回りしよ～",
            "体を動かしてリフレッシュ、それからまたがんばろ～",
            "立って、深呼吸して、ぐーっと伸び～！🙆",
        ],
        (HealthStand, En) => &[
            "Stand up and stretch, {you} — your back will thank you~ 🙆",
            "Take a few steps to get the blood moving!",
            "Roll your shoulders, turn your neck a bit~ Let's do it together!",
            "You've been sitting an hour, take a little walk~",
            "Move around a bit, then come back fresh~",
            "Stand, breathe in, and stretch up high! 🙆",
        ],

        (HealthMulti, Vi) => &[
            "Nghỉ một chút nha {you}: uống ngụm nước, đứng dậy vươn vai, nhìn ra xa cho mắt đỡ mỏi~ 🌿",
            "{You} ơi, giải lao 2 phút nè: nước một ngụm, vai xoay vài vòng, mắt nhìn xa xa~",
            "Tới giờ nạp năng lượng rồi: đứng lên, uống nước, cho mắt nghỉ nha! 🌿",
            "{Me} đề nghị một pha nghỉ ngơi: vươn vai + uống nước + nhìn ra cửa sổ~",
            "Làm lâu rồi đó, nghỉ xíu đi {you}: nước, vươn vai, nghỉ mắt đủ cả nha~",
            "Pause nhẹ nào {you}! Đứng dậy, uống nước, chớp mắt nhìn xa~ 🌿",
        ],
        (HealthMulti, Ja) => &[
            "ちょっと休憩しよ：お水をひとくち、伸びをして、遠くを見て目も休めて～🌿",
            "{you}、2分だけ休憩：お水、肩まわし、遠くを見る～",
            "エネルギー補給タイム：立って、お水飲んで、目を休めよう！🌿",
            "休憩セットを提案します：伸び＋お水＋窓の外～",
            "がんばりすぎだよ、ひと休み：お水、ストレッチ、目の休憩ぜんぶね～",
            "ちょっとストップ！立って、お水、遠くを見て～🌿",
        ],
        (HealthMulti, En) => &[
            "Quick break, {you}: a sip of water, a big stretch, and rest your eyes~ 🌿",
            "Two-minute pause: water, shoulder rolls, look far away~",
            "Recharge time: stand up, drink some water, rest your eyes! 🌿",
            "I propose a break combo: stretch + water + window gazing~",
            "You've worked a while — water, stretch, eye rest, all of it~",
            "Little pause! Stand, sip, and blink at something far away~ 🌿",
        ],

        // ── Khen khi xong việc ──────────────────────────────────────────────
        (TaskDone, Vi) => &[
            "Xong \"{task}\" rồi! {You} giỏi quá~ ✨",
            "Thêm một việc xong nè: \"{task}\"! 👏",
            "\"{task}\" ✔️ — {me} thấy hết á, {you} làm tốt lắm!",
            "Yay, \"{task}\" xong rồi! Thưởng cho {you} một tràng pháo tay~",
            "Gạch được \"{task}\" rồi, nhẹ cả người ha {you}~",
            "\"{task}\" xong! Cứ đà này là hết việc sớm thôi~ 💪",
        ],
        (TaskDone, Ja) => &[
            "「{task}」完了！{you}、すごい～✨",
            "ひとつ終わったね：「{task}」！👏",
            "「{task}」✔️ ちゃんと見てたよ、よくがんばったね！",
            "やった、「{task}」おわり！拍手～",
            "「{task}」消せたね、すっきり～",
            "「{task}」完了！この調子ならすぐ全部終わるよ～💪",
        ],
        (TaskDone, En) => &[
            "\"{task}\" done! Nice work, {you}~ ✨",
            "One more off the list: \"{task}\"! 👏",
            "\"{task}\" ✔️ — I saw that, great job!",
            "Yay, \"{task}\" is finished! Round of applause~",
            "Crossed off \"{task}\" — feels good, right?",
            "\"{task}\" done! At this pace you'll be free in no time~ 💪",
        ],

        (TaskDoneEarly, Vi) => &[
            "Xong \"{task}\" trước hạn luôn! {You} đỉnh thật sự~ 🌟",
            "\"{task}\" xong sớm hơn hạn nè, {me} nể {you} ghê!",
        ],
        (TaskDoneEarly, Ja) => &[
            "「{task}」締め切り前に完了！{you}、さすが～🌟",
            "「{task}」予定より早く終わったね、すごい！",
        ],
        (TaskDoneEarly, En) => &[
            "\"{task}\" done before the deadline! You're amazing~ 🌟",
            "\"{task}\" finished early — I'm impressed!",
        ],

        (AllDone, Vi) => &[
            "\"{task}\" là việc cuối rồi! Hết sạch việc luôn, {you} tuyệt vời quá~ 🎉",
            "Danh sách trống trơn rồi nè! Xong \"{task}\" là hết việc, nghỉ ngơi thôi {you}~ 🎉",
        ],
        (AllDone, Ja) => &[
            "「{task}」で最後だよ！全部終わった、{you}すごすぎ～🎉",
            "リストが空っぽになった！「{task}」でおしまい、ゆっくり休んでね～🎉",
        ],
        (AllDone, En) => &[
            "\"{task}\" was the last one! Everything's done, {you} — amazing~ 🎉",
            "The list is empty! \"{task}\" was the final task, time to relax~ 🎉",
        ],

        (DoneMilestone, Vi) => &[
            "Hôm nay {you} xong {count} việc rồi đó! Năng suất ghê~ 🏆",
            "{count} việc trong một ngày! {Me} tự hào về {you} lắm~ 🏆",
        ],
        (DoneMilestone, Ja) => &[
            "今日はもう{count}個も終わらせたよ！すごい～🏆",
            "1日で{count}個！{you}のこと、誇りに思うよ～🏆",
        ],
        (DoneMilestone, En) => &[
            "That's {count} tasks done today! So productive~ 🏆",
            "{count} tasks in one day! I'm so proud of you~ 🏆",
        ],

        // ── Ghi chú dưới câu trả lời chat ───────────────────────────────────
        (TaskSavedDue, Vi) => &["📝 Đã lưu task: {task} (hạn {time})"],
        (TaskSavedDue, Ja) => &["📝 タスクを保存したよ：{task}（締め切り {time}）"],
        (TaskSavedDue, En) => &["📝 Task saved: {task} (due {time})"],

        (DueUnparsed, Vi) => &["📝 Đã lưu task: {task} (chưa hiểu hạn chót, đặt lại trong bảng Công việc nha)"],
        (DueUnparsed, Ja) => &["📝 タスクを保存したよ：{task}（締め切りが読めなかったので、タスク画面で設定してね）"],
        (DueUnparsed, En) => &["📝 Task saved: {task} (couldn't read the deadline — set it in the Tasks panel)"],

        (DoneNote, Vi) => &["✅ Đã đánh dấu xong: {task}"],
        (DoneNote, Ja) => &["✅ 完了にしたよ：{task}"],
        (DoneNote, En) => &["✅ Marked done: {task}"],

        (DeletedNote, Vi) => &["🗑 Đã xoá task: {task}"],
        (DeletedNote, Ja) => &["🗑 タスクを削除したよ：{task}"],
        (DeletedNote, En) => &["🗑 Task deleted: {task}"],

        (TaskNotFound, Vi) => &["❓ Không tìm thấy task nào khớp \"{query}\", chưa đổi gì cả."],
        (TaskNotFound, Ja) => &["❓「{query}」に合うタスクが見つからなかったので、何も変えてないよ。"],
        (TaskNotFound, En) => &["❓ No task matches \"{query}\", so nothing was changed."],

        (TaskAmbiguous, Vi) => &["❓ Có nhiều task khớp \"{query}\", chưa đổi gì. Nói rõ hơn giúp {me} nha?"],
        (TaskAmbiguous, Ja) => &["❓「{query}」に合うタスクが複数あるので、何も変えてないよ。もう少し詳しく教えて？"],
        (TaskAmbiguous, En) => &["❓ Several tasks match \"{query}\", so nothing was changed. Which one?"],

        (ToastTest, Vi) => &["Thông báo thử nè! Thấy dòng này là {me} nhắc hạn chót được rồi~ 🔔"],
        (ToastTest, Ja) => &["テスト通知だよ！これが見えたら締め切りのお知らせはOK～🔔"],
        (ToastTest, En) => &["Test notification! If you can see this, deadline alerts work~ 🔔"],
    }
}

/// Chọn biến thể thứ mấy: ngẫu nhiên theo `seed`, tránh lặp đúng câu vừa nói.
pub fn pick_index(n: usize, seed: u64, last: Option<usize>) -> usize {
    if n <= 1 {
        return 0;
    }
    let i = (seed % n as u64) as usize;
    if Some(i) == last { (i + 1) % n } else { i }
}

static LAST: Mutex<Option<HashMap<(Key, Lang), usize>>> = Mutex::new(None);

fn seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64 ^ (d.as_secs().rotate_left(17)))
        .unwrap_or(0)
}

/// Một câu loại `key` theo ngôn ngữ + xưng hô của `a`, không lặp câu vừa nói.
pub fn line(key: Key, a: &Address, vars: &[(&str, &str)]) -> String {
    let list = templates(key, a.lang);
    let mut guard = LAST.lock().unwrap_or_else(|e| e.into_inner());
    let last = guard.get_or_insert_with(HashMap::new);
    let i = pick_index(list.len(), seed(), last.get(&(key, a.lang)).copied());
    last.insert((key, a.lang), i);
    persona::render(list[i], a, vars)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(lang: Lang) -> Address {
        Address { lang, me: "em".into(), you: "anh".into(), pet: "Mây".into() }
    }

    #[test]
    fn every_key_has_all_languages() {
        for &key in Key::ALL {
            for lang in Lang::ALL {
                let n = templates(key, lang).len();
                assert!(n >= key.min_variants(), "{key:?}/{lang:?}: {n} câu, cần ≥ {}", key.min_variants());
            }
        }
    }

    #[test]
    fn templates_use_only_known_placeholders() {
        let known = ["me", "you", "Me", "You", "name"];
        for &key in Key::ALL {
            for lang in Lang::ALL {
                for t in templates(key, lang) {
                    let mut rest = *t;
                    while let Some(open) = rest.find('{') {
                        let close = rest[open..].find('}').map(|c| open + c).unwrap_or_else(|| {
                            panic!("{key:?}/{lang:?}: ngoặc không đóng trong {t}")
                        });
                        let var = &rest[open + 1..close];
                        assert!(
                            known.contains(&var) || key.vars().contains(&var),
                            "{key:?}/{lang:?}: biến lạ {{{var}}} trong {t}"
                        );
                        rest = &rest[close + 1..];
                    }
                }
            }
        }
    }

    #[test]
    fn rendered_lines_have_no_leftover_braces() {
        for &key in Key::ALL {
            let vars: Vec<(&str, &str)> = key.vars().iter().map(|v| (*v, "x")).collect();
            for lang in Lang::ALL {
                for t in templates(key, lang) {
                    let s = persona::render(t, &addr(lang), &vars);
                    assert!(!s.contains('{') && !s.contains('}'), "{key:?}/{lang:?}: {s}");
                }
            }
        }
    }

    #[test]
    fn name_lines_mention_the_name() {
        for lang in Lang::ALL {
            for t in templates(Key::ThanksName, lang).iter().chain(templates(Key::Renamed, lang)) {
                assert!(t.contains("{name}"), "{lang:?}: {t}");
            }
        }
    }

    #[test]
    fn picking_avoids_repeats() {
        assert_eq!(pick_index(1, 7, Some(0)), 0);
        assert_eq!(pick_index(0, 7, None), 0);
        assert_eq!(pick_index(4, 6, None), 2);
        assert_eq!(pick_index(4, 6, Some(2)), 3);
        assert_eq!(pick_index(4, 7, Some(3)), 0);
        for seed in 0..50 {
            let a = pick_index(3, seed, None);
            assert_ne!(pick_index(3, seed, Some(a)), a);
        }
    }

    #[test]
    fn line_uses_address() {
        let s = line(Key::ThanksName, &addr(Lang::Vi), &[]);
        assert!(s.contains("Mây") && s.contains("anh"), "{s}");
        let s = line(Key::TaskSaved, &addr(Lang::En), &[("task", "report")]);
        assert_eq!(s, "📝 Task saved: report");
    }
}
