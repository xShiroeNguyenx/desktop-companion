// 🍬 Kẹo + Bộ sưu tập (PLAN.md mục 14, Phase 17). Kẹo là tiền tiêu được, cộng
// song song điểm thân thiết: mỗi lần bond::award cộng được `gain` điểm thì kẹo
// cộng `gain × candy_mult`, nên trần mỗi ngày kế thừa từ bond.rs. Kẹo đổi được
// đồ trang trí (skin widget, hiệu ứng, bộ nhân vật bám mép, bộ mặt Miu) — không
// bao giờ là chức năng, và không bao giờ bán bằng tiền thật.
//
// Trạng thái nằm ở collection.json riêng (store.rs), KHÔNG trong Settings: form
// Cài đặt chép lại các trường "không có trên form" từ bản cũ, thêm trường mà
// quên một chỗ là mất đồ đã mở. Phần thuần ở trên, phần Tauri (file, sự kiện,
// áp dụng ngay) ở cuối file.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::bond::Event;

// ─── Catalog ─────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    /// Màu của 3 widget ghim (app/skins.css).
    Skin,
    /// Bộ nhân vật bám mép (sidegirl.rs).
    SideSet,
    /// Bộ mặt Miu mini trên widget (thư mục chứa rps-<face>.png).
    MiuSet,
    /// Hiệu ứng trên pet (pháo hoa, pháo giấy); mở rồi bật / tắt được.
    Effect,
    /// Bộ câu thoại hiếm (Phase 18).
    LinePack,
    /// Kiểu bong bóng thoại của pet (companion.css body[data-bubble]).
    Bubble,
    /// Biểu tượng hiện khi bôi chữ (thay 🌸, flower.html).
    Flower,
    /// Model Live2D (models.rs). Model đang dùng lưu ở `Settings.pet_model`, không ở đây.
    Model,
    /// Trang trí trên mép 3 widget ghim (app/widget-deco.css [data-deco]).
    WidgetDeco,
}

impl Category {
    pub fn code(self) -> &'static str {
        match self {
            Category::Skin => "skin",
            Category::SideSet => "side_set",
            Category::MiuSet => "miu_set",
            Category::Effect => "effect",
            Category::LinePack => "line_pack",
            Category::Bubble => "bubble",
            Category::Flower => "flower",
            Category::Model => "model",
            Category::WidgetDeco => "widget_deco",
        }
    }

    pub fn from_code(s: &str) -> Option<Category> {
        [
            Category::Skin,
            Category::SideSet,
            Category::MiuSet,
            Category::Effect,
            Category::LinePack,
            Category::Bubble,
            Category::Flower,
            Category::Model,
            Category::WidgetDeco,
        ]
            .into_iter()
            .find(|c| c.code() == s)
    }

    /// Nhóm có "món đang dùng" (chọn một trong các món đã có).
    fn selectable(self) -> bool {
        matches!(self, Category::Skin | Category::SideSet | Category::MiuSet | Category::Bubble | Category::Flower | Category::WidgetDeco)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rarity {
    Common,
    Rare,
    Epic,
    /// Model Live2D (PLAN.md 16.4): giá theo từng món, tỉ lệ gacha theo từng món (gacha.rs).
    Legendary,
}

impl Rarity {
    /// Giá đổi thẳng: ~3 ngày / ~1 tuần / ~2 tuần với ~200 kẹo/ngày. Đắt gấp 6–30
    /// lượt gacha (gacha.rs) để quay là đường rẻ, đổi thẳng là đường chắc.
    /// Chỉ test dùng: kiểm mọi món trong CATALOG có giá khớp hạng. Huyền thoại
    /// không có một giá chung, mỗi món một giá trong khoảng này.
    #[cfg(test)]
    pub fn prices(self) -> std::ops::RangeInclusive<u32> {
        match self {
            Rarity::Common => 600..=600,
            Rarity::Rare => 1500..=1500,
            Rarity::Epic => 3000..=3000,
            Rarity::Legendary => 5000..=10000,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Rarity::Common => "common",
            Rarity::Rare => "rare",
            Rarity::Epic => "epic",
            Rarity::Legendary => "legendary",
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Item {
    pub id: &'static str,
    pub cat: Category,
    pub rarity: Rarity,
    /// 0 = món mặc định (có sẵn, không lưu vào `owned`).
    pub price: u32,
    /// Tên theo thứ tự vi, ja, en (lang_index).
    pub name: [&'static str; 3],
    pub desc: [&'static str; 3],
    /// Skin: id trong skins.css · MiuSet: thư mục ảnh · SideSet: id bộ (sidegirl::SETS) · Effect: loại fx
    /// · Model: id trong models::BUILTIN · WidgetDeco: id trong widget-deco.css.
    pub asset: &'static str,
}

pub const CATALOG: &[Item] = &[
    Item {
        id: "skin.matcha",
        cat: Category::Skin,
        rarity: Rarity::Common,
        price: 600,
        name: ["Trà xanh", "抹茶", "Matcha"],
        desc: [
            "Xanh trà sữa dịu mắt.",
            "やさしい抹茶ミルク色。",
            "Soft matcha-latte green.",
        ],
        asset: "matcha",
    },
    Item {
        id: "skin.cotton",
        cat: Category::Skin,
        rarity: Rarity::Common,
        price: 600,
        name: ["Kẹo bông", "わたあめ", "Cotton candy"],
        desc: [
            "Hồng và xanh pastel như kẹo bông.",
            "ピンクと水色のパステル。",
            "Pastel pink and sky blue.",
        ],
        asset: "cotton",
    },
    Item {
        id: "skin.latte",
        cat: Category::Skin,
        rarity: Rarity::Common,
        price: 600,
        name: ["Cà phê sữa", "カフェラテ", "Café latte"],
        desc: [
            "Nâu kem ấm áp.",
            "あたたかいミルクブラウン。",
            "Warm creamy brown.",
        ],
        asset: "latte",
    },
    Item {
        id: "skin.neon",
        cat: Category::Skin,
        rarity: Rarity::Common,
        price: 600,
        name: ["Đêm neon", "ネオンの夜", "Neon night"],
        desc: [
            "Nền tối, viền hồng xanh phát sáng.",
            "暗い背景に光るピンクと水色。",
            "Dark with glowing pink and cyan.",
        ],
        asset: "neon",
    },
    Item {
        id: "fx.hearts",
        cat: Category::Effect,
        rarity: Rarity::Rare,
        price: 1500,
        name: ["Mưa tim", "ハートの雨", "Heart shower"],
        desc: [
            "Tim bay lên khi vuốt đầu và khi lên cấp thân thiết.",
            "なでなでやなかよしレベルアップでハートが舞います。",
            "Hearts float up on headpats and friendship level-ups.",
        ],
        asset: "hearts",
    },
    Item {
        id: "fx.bubbles",
        cat: Category::Effect,
        rarity: Rarity::Rare,
        price: 1500,
        name: ["Bong bóng xà phòng", "しゃぼん玉", "Soap bubbles"],
        desc: [
            "Bong bóng lơ lửng khi pet chào buổi sáng.",
            "朝のあいさつでしゃぼん玉がふわふわ。",
            "Bubbles drift up with the morning greeting.",
        ],
        asset: "bubbles",
    },
    Item {
        id: "fx.shooting",
        cat: Category::Effect,
        rarity: Rarity::Rare,
        price: 1500,
        name: ["Sao băng", "流れ星", "Shooting star"],
        desc: [
            "Sao băng vụt qua khi xong việc sau 22 giờ.",
            "22時以降にタスクを終えると流れ星。",
            "A shooting star when you finish a task after 10 pm.",
        ],
        asset: "shooting",
    },
    Item {
        id: "bubble.classic",
        cat: Category::Bubble,
        rarity: Rarity::Common,
        price: 0,
        name: ["Bong bóng hồng", "ピンクの吹き出し", "Pink bubble"],
        desc: [
            "Bong bóng thoại quen thuộc.",
            "いつもの吹き出し。",
            "The familiar speech bubble.",
        ],
        asset: "classic",
    },
    Item {
        id: "bubble.cloud",
        cat: Category::Bubble,
        rarity: Rarity::Common,
        price: 600,
        name: ["Đám mây", "くも", "Cloud"],
        desc: [
            "Bong bóng mây trắng bồng bềnh.",
            "ふわふわの白い雲。",
            "A fluffy white cloud.",
        ],
        asset: "cloud",
    },
    Item {
        id: "bubble.heart",
        cat: Category::Bubble,
        rarity: Rarity::Common,
        price: 600,
        name: ["Trái tim", "ハート", "Heart"],
        desc: [
            "Viền hồng đậm, góc có tim nhỏ.",
            "濃いピンクの縁に小さなハート。",
            "Deep pink border with little hearts.",
        ],
        asset: "heart",
    },
    Item {
        id: "bubble.lace",
        cat: Category::Bubble,
        rarity: Rarity::Common,
        price: 600,
        name: ["Ren gothic", "ゴシックレース", "Gothic lace"],
        desc: [
            "Nền tím than, viền ren tím.",
            "紫紺に紫のレース縁。",
            "Dark violet with a lace trim.",
        ],
        asset: "lace",
    },
    Item {
        id: "bubble.scroll",
        cat: Category::Bubble,
        rarity: Rarity::Common,
        price: 600,
        name: ["Cuộn giấy", "巻物", "Scroll"],
        desc: [
            "Giấy cũ màu kem, chữ nâu.",
            "クリーム色の古い紙に茶色の文字。",
            "Cream parchment with brown text.",
        ],
        asset: "scroll",
    },
    Item {
        id: "flower.sakura",
        cat: Category::Flower,
        rarity: Rarity::Common,
        price: 0,
        name: ["Hoa anh đào", "さくら", "Sakura"],
        desc: [
            "Bông hoa 🌸 quen thuộc khi bôi chữ.",
            "文字を選ぶといつもの🌸。",
            "The familiar 🌸 when you select text.",
        ],
        asset: "sakura",
    },
    Item {
        id: "flower.star",
        cat: Category::Flower,
        rarity: Rarity::Common,
        price: 600,
        name: ["Ngôi sao", "ほし", "Star"],
        desc: [
            "Bôi chữ hiện ngôi sao ⭐.",
            "文字を選ぶと⭐。",
            "A ⭐ appears when you select text.",
        ],
        asset: "star",
    },
    Item {
        id: "flower.clover",
        cat: Category::Flower,
        rarity: Rarity::Common,
        price: 600,
        name: ["Cỏ bốn lá", "クローバー", "Clover"],
        desc: [
            "Bôi chữ hiện cỏ bốn lá 🍀.",
            "文字を選ぶと🍀。",
            "A 🍀 appears when you select text.",
        ],
        asset: "clover",
    },
    Item {
        id: "flower.paw",
        cat: Category::Flower,
        rarity: Rarity::Common,
        price: 600,
        name: ["Dấu chân mèo", "肉球", "Paw print"],
        desc: [
            "Bôi chữ hiện dấu chân 🐾.",
            "文字を選ぶと🐾。",
            "A 🐾 appears when you select text.",
        ],
        asset: "paw",
    },
    Item {
        id: "flower.snow",
        cat: Category::Flower,
        rarity: Rarity::Common,
        price: 600,
        name: ["Bông tuyết", "雪の結晶", "Snowflake"],
        desc: [
            "Bôi chữ hiện bông tuyết ❄️.",
            "文字を選ぶと❄️。",
            "A ❄️ appears when you select text.",
        ],
        asset: "snow",
    },
    Item {
        id: "skin.lavender",
        cat: Category::Skin,
        rarity: Rarity::Common,
        price: 0,
        name: ["Oải hương", "ラベンダー", "Lavender"],
        desc: ["Màu tím quen thuộc.", "いつもの紫。", "The familiar purple."],
        asset: "lavender",
    },
    Item {
        id: "skin.sakura",
        cat: Category::Skin,
        rarity: Rarity::Common,
        price: 600,
        name: ["Hoa anh đào", "さくら", "Sakura"],
        desc: ["Hồng phấn, chữ tím đậm.", "淡いピンクに濃い紫の文字。", "Soft pink with deep-purple text."],
        asset: "sakura",
    },
    Item {
        id: "skin.night",
        cat: Category::Skin,
        rarity: Rarity::Common,
        price: 600,
        name: ["Đêm sao", "星空", "Night sky"],
        desc: ["Xanh chàm sang tím than.", "藍色から濃紺へ。", "Indigo fading into deep violet."],
        asset: "night",
    },
    Item {
        id: "skin.mint",
        cat: Category::Skin,
        rarity: Rarity::Common,
        price: 600,
        name: ["Bạc hà", "ミント", "Mint"],
        desc: ["Xanh ngọc nhạt, mát mắt.", "さわやかなミントグリーン。", "Cool, pale mint green."],
        asset: "mint",
    },
    Item {
        id: "skin.sunset",
        cat: Category::Skin,
        rarity: Rarity::Common,
        price: 600,
        name: ["Hoàng hôn", "夕焼け", "Sunset"],
        desc: ["Cam đào sang tím hồng.", "桃色の橙から藤色へ。", "Peach orange into rosy violet."],
        asset: "sunset",
    },
    Item {
        id: "skin.ocean",
        cat: Category::Skin,
        rarity: Rarity::Common,
        price: 600,
        name: ["Biển", "海", "Ocean"],
        desc: ["Xanh biển sâu.", "深い海の青。", "Deep sea blue."],
        asset: "ocean",
    },
    Item {
        id: "fx.fireworks",
        cat: Category::Effect,
        rarity: Rarity::Rare,
        price: 1500,
        name: ["Pháo hoa", "花火", "Fireworks"],
        desc: [
            "Bắn pháo hoa khi xong hết việc trong ngày.",
            "その日のタスクを全部終えたら花火。",
            "Fireworks when every task of the day is done.",
        ],
        asset: "fireworks",
    },
    Item {
        id: "fx.confetti",
        cat: Category::Effect,
        rarity: Rarity::Rare,
        price: 1500,
        name: ["Pháo giấy", "紙吹雪", "Confetti"],
        desc: [
            "Tung pháo giấy khi xong một phiên Tập trung.",
            "集中セッションを終えたら紙吹雪。",
            "Confetti when a focus session ends.",
        ],
        asset: "confetti",
    },
    Item {
        id: "fx.petals",
        cat: Category::Effect,
        rarity: Rarity::Rare,
        price: 1500,
        name: ["Mưa hoa anh đào", "桜吹雪", "Sakura shower"],
        desc: [
            "Cánh hoa anh đào rơi mỗi lần xong một việc.",
            "タスクをひとつ終えるたびに桜の花びらが舞います。",
            "Cherry blossom petals fall every time you finish a task.",
        ],
        asset: "petals",
    },
    Item {
        id: "side.bunny",
        cat: Category::SideSet,
        rarity: Rarity::Epic,
        price: 0,
        name: ["Bunny Lounge", "バニーラウンジ", "Bunny Lounge"],
        desc: [
            "Hai cô thỏ tóc hồng và tóc tím tựa bên các khung của app.",
            "ピンクと薄紫の髪のうさぎ娘ふたりがアプリの縁に寄りかかります。",
            "Two bunny girls, pink-haired and lilac-haired, lean on the app's panels.",
        ],
        asset: "bunny",
    },
    Item {
        id: "side.neko",
        cat: Category::SideSet,
        rarity: Rarity::Epic,
        price: 3000,
        name: ["Neko Lounge", "ネコラウンジ", "Neko Lounge"],
        desc: [
            "Cô mèo gothic tím và cô mèo punk áo hoodie tai mèo tựa bên các khung của app.",
            "ゴシックな紫の猫娘と猫耳パーカーのパンクな猫娘がアプリの縁に寄りかかります。",
            "A gothic purple cat girl and a punk cat girl in a cat-ear hoodie lean on the app's panels.",
        ],
        asset: "neko",
    },
    Item {
        id: "side.fox",
        cat: Category::SideSet,
        rarity: Rarity::Epic,
        price: 3000,
        name: ["Hồ ly", "狐の社", "Fox Lounge"],
        desc: [
            "Cô hồ ly miko chuông vàng và cô hồ ly steampunk tựa bên các khung của app.",
            "鈴の巫女狐とスチームパンクな狐娘がアプリの縁に寄りかかります。",
            "A shrine-maiden fox with golden bells and a steampunk fox girl lean on the app's panels.",
        ],
        asset: "fox",
    },
    Item {
        id: "side.snow",
        cat: Category::SideSet,
        rarity: Rarity::Epic,
        price: 3000,
        name: ["Xứ Tuyết", "雪の国", "Snow Lounge"],
        desc: [
            "Cô mèo băng pha lê, cô thỏ tuyết và cáo tuyết tựa bên các khung của app.",
            "氷の猫娘・雪うさぎ・雪狐がアプリの縁に寄りかかります。",
            "An ice-crystal cat girl, a snow bunny and a snow fox lean on the app's panels.",
        ],
        asset: "snow",
    },
    Item {
        id: "model.gothic-aristocrat",
        cat: Category::Model,
        rarity: Rarity::Legendary,
        price: 10000,
        name: ["Gothic Aristocrat", "ゴシック・アリストクラット", "Gothic Aristocrat"],
        desc: [
            "Tiểu thư gothic quý tộc thay nhân vật chính trên màn hình: váy ren đen, đủ biểu cảm và động tác.",
            "黒レースのドレスをまとうゴシックな令嬢。画面のメインキャラクターになり、表情もモーションもそろっています。",
            "A gothic young noble in black lace becomes your main on-screen character, with full expressions and motions.",
        ],
        asset: "gothic-aristocrat",
    },
    Item {
        id: "model.pink-angel",
        cat: Category::Model,
        rarity: Rarity::Legendary,
        price: 10000,
        name: ["Pink Angel", "ピンク・エンジェル", "Pink Angel"],
        desc: [
            "Cô gái tai mèo tóc hồng đeo kính, váy bèo nơ tim và thỏ bông, thay nhân vật chính trên màn hình với đủ biểu cảm và động tác.",
            "ハートとうさぎで飾ったフリルドレスの、眼鏡をかけたピンク髪の猫耳少女。画面のメインキャラクターになり、表情もモーションもそろっています。",
            "A pink-haired cat girl in glasses and a frilly dress of hearts and plush bunnies becomes your main on-screen character, with full expressions and motions.",
        ],
        asset: "pink-angel",
    },
    Item {
        id: "model.snow-lolita",
        cat: Category::Model,
        rarity: Rarity::Legendary,
        price: 5000,
        name: ["Snow Lolita", "スノー・ロリータ", "Snow Lolita"],
        desc: [
            "Cô gái tai mèo tóc bạch kim đeo kính, áo lông trắng điểm bông tuyết và pha lê băng, thay nhân vật chính trên màn hình với đủ biểu cảm và động tác.",
            "雪の結晶と氷のクリスタルで飾った白いファーコートの、眼鏡をかけた銀髪の猫耳少女。画面のメインキャラクターになり、表情もモーションもそろっています。",
            "A silver-haired cat girl in glasses and a white fur coat of snowflakes and ice crystals becomes your main on-screen character, with full expressions and motions.",
        ],
        asset: "snow-lolita",
    },
    Item {
        id: "model.snow-aristocrat",
        cat: Category::Model,
        rarity: Rarity::Legendary,
        price: 10000,
        name: ["Snow Aristocrat", "スノー・アリストクラット", "Snow Aristocrat"],
        desc: [
            "Tiểu thư tai mèo tóc bạch kim xoăn dài, váy bèo trắng xanh điểm bông tuyết và pha lê băng, thay nhân vật chính trên màn hình với đủ biểu cảm và động tác.",
            "雪の結晶と氷のクリスタルで飾った白と青のフリルドレスの、長い巻き髪が銀色の猫耳令嬢。画面のメインキャラクターになり、表情もモーションもそろっています。",
            "A silver-haired cat-eared noble with long curls, in a white and blue frilled dress of snowflakes and ice crystals, becomes your main on-screen character, with full expressions and motions.",
        ],
        asset: "snow-aristocrat",
    },
    Item {
        id: "model.gothic-lolita",
        cat: Category::Model,
        rarity: Rarity::Legendary,
        price: 5000,
        name: ["Gothic Lolita", "ゴシック・ロリータ", "Gothic Lolita"],
        desc: [
            "Cô gái tai mèo tóc tím lavender đeo kính, váy gothic lolita đen tím điểm tim và thỏ bông, thay nhân vật chính trên màn hình với đủ biểu cảm và động tác.",
            "ハートとうさぎで飾った黒と紫のゴスロリドレスの、眼鏡をかけたラベンダー髪の猫耳少女。画面のメインキャラクターになり、表情もモーションもそろっています。",
            "A lavender-haired cat girl in glasses and a black and purple gothic lolita dress of hearts and plush bunnies becomes your main on-screen character, with full expressions and motions.",
        ],
        asset: "gothic-lolita",
    },
    Item {
        id: "model.snow-dancer",
        cat: Category::Model,
        rarity: Rarity::Legendary,
        price: 5000,
        name: ["Snow Dancer", "スノー・ダンサー", "Snow Dancer"],
        desc: [
            "Vũ công đường phố tóc nâu ánh xanh, mũ lưỡi trai bông tuyết, áo khoác trắng và chân váy đen xích bạc, thay nhân vật chính trên màn hình với đủ biểu cảm và động tác.",
            "雪の結晶のキャップに白いジャケット、チェーン付きの黒いスカートをまとった、青みがかった茶髪のストリートダンサー。画面のメインキャラクターになり、表情もモーションもそろっています。",
            "A street dancer with blue-tipped brown hair, a snowflake cap, a white jacket and a chained black skirt becomes your main on-screen character, with full expressions and motions.",
        ],
        asset: "snow-dancer",
    },
    Item {
        id: "model.gothic-pastel",
        cat: Category::Model,
        rarity: Rarity::Legendary,
        price: 7500,
        name: ["Gothic Pastel", "ゴシック・パステル", "Gothic Pastel"],
        desc: [
            "Cô gái tai mèo tóc bạch kim ánh tím đeo kính, váy bèo trắng điểm nơ đen tím và thỏ bông, thay nhân vật chính trên màn hình với đủ biểu cảm và động tác.",
            "黒と紫のリボンとうさぎで飾った白いフリル服の、眼鏡をかけた薄紫がかった銀髪の猫耳少女。画面のメインキャラクターになり、表情もモーションもそろっています。",
            "A lilac-silver-haired cat girl in glasses and white frills with black and purple bows and plush bunnies becomes your main on-screen character, with full expressions and motions.",
        ],
        asset: "gothic-pastel",
    },
    Item {
        id: "model.pink-pastel",
        cat: Category::Model,
        rarity: Rarity::Legendary,
        price: 7500,
        name: ["Pink Pastel", "ピンク・パステル", "Pink Pastel"],
        desc: [
            "Cô gái tai mèo tóc hồng phấn đeo kính, váy bèo trắng điểm nơ hồng, hoa anh đào và thỏ bông, thay nhân vật chính trên màn hình với đủ biểu cảm và động tác.",
            "ピンクのリボンと桜とうさぎで飾った白いフリル服の、眼鏡をかけた淡いピンク髪の猫耳少女。画面のメインキャラクターになり、表情もモーションもそろっています。",
            "A pastel-pink-haired cat girl in glasses and white frills with pink bows, cherry blossoms and plush bunnies becomes your main on-screen character, with full expressions and motions.",
        ],
        asset: "pink-pastel",
    },
    Item {
        id: "model.snow-pastel",
        cat: Category::Model,
        rarity: Rarity::Legendary,
        price: 7500,
        name: ["Snow Pastel", "スノー・パステル", "Snow Pastel"],
        desc: [
            "Cô gái tai mèo tóc bạch kim đeo kính, váy bèo trắng tay áo lông, điểm nơ xanh bông tuyết, thay nhân vật chính trên màn hình với đủ biểu cảm và động tác.",
            "雪の結晶の青いリボンで飾った白いフリル服とファーの袖の、眼鏡をかけた銀髪の猫耳少女。画面のメインキャラクターになり、表情もモーションもそろっています。",
            "A silver-haired cat girl in glasses, white frills and fur sleeves with blue snowflake bows becomes your main on-screen character, with full expressions and motions.",
        ],
        asset: "snow-pastel",
    },
    Item {
        id: "wdeco.none",
        cat: Category::WidgetDeco,
        rarity: Rarity::Common,
        price: 0,
        name: ["Không trang trí", "デコなし", "No decor"],
        desc: [
            "Widget để trơn, không trang trí.",
            "ウィジェットに飾りをつけません。",
            "Plain widgets with no decor.",
        ],
        asset: "none",
    },
    Item {
        id: "wdeco.ribbon",
        cat: Category::WidgetDeco,
        rarity: Rarity::Common,
        price: 600,
        name: ["Nơ ruy băng", "リボン", "Ribbon bows"],
        desc: [
            "Hai chiếc nơ 🎀 cài trên góc các widget.",
            "ウィジェットの角に🎀のリボンがふたつ。",
            "Two 🎀 bows pinned to the widgets' corners.",
        ],
        asset: "ribbon",
    },
    Item {
        id: "wdeco.tape",
        cat: Category::WidgetDeco,
        rarity: Rarity::Common,
        price: 600,
        name: ["Băng dính washi", "マスキングテープ", "Washi tape"],
        desc: [
            "Mẩu băng dính hoa văn dán widget lên màn hình như dán sổ tay.",
            "手帳みたいに柄入りのマスキングテープでウィジェットを留めます。",
            "Patterned washi tape sticks the widgets to your screen like a planner page.",
        ],
        asset: "tape",
    },
    Item {
        id: "wdeco.sakura",
        cat: Category::WidgetDeco,
        rarity: Rarity::Common,
        price: 600,
        name: ["Hoa anh đào", "桜", "Sakura"],
        desc: [
            "Chùm hoa 🌸 ở góc widget, cánh hoa rơi chầm chậm.",
            "ウィジェットの角に🌸、花びらがゆっくり舞います。",
            "🌸 blossoms on the widgets' corners, with petals drifting down.",
        ],
        asset: "sakura",
    },
    Item {
        id: "wdeco.lace",
        cat: Category::WidgetDeco,
        rarity: Rarity::Rare,
        price: 1500,
        name: ["Viền ren", "レース", "Lace trim"],
        desc: [
            "Viền ren trắng đính ngọc trai trên mép widget.",
            "真珠つきの白いレースでウィジェットの縁を飾ります。",
            "White lace with pearls trims the widgets' edges.",
        ],
        asset: "lace",
    },
    Item {
        id: "wdeco.stars",
        cat: Category::WidgetDeco,
        rarity: Rarity::Rare,
        price: 1500,
        name: ["Dây đèn sao", "星のガーランド", "Star garland"],
        desc: [
            "Dây đèn ngôi sao võng trên đầu widget, nhấp nháy lung linh.",
            "ウィジェットの上に星のガーランド、きらきら瞬きます。",
            "A garland of twinkling stars hangs across the widgets.",
        ],
        asset: "stars",
    },
    Item {
        id: "wdeco.snow",
        cat: Category::WidgetDeco,
        rarity: Rarity::Rare,
        price: 1500,
        name: ["Tuyết phủ", "雪化粧", "Snowcap"],
        desc: [
            "Lớp tuyết phủ trên mép widget, bông tuyết rơi nhẹ.",
            "ウィジェットの上に雪が積もり、雪がちらちら降ります。",
            "Snow piles on top of the widgets while flakes fall gently.",
        ],
        asset: "snow",
    },
    Item {
        id: "wdeco.holo",
        cat: Category::WidgetDeco,
        rarity: Rarity::Epic,
        price: 3000,
        name: ["Viền ánh kim", "ホロフレーム", "Holo frame"],
        desc: [
            "Viền cầu vồng đổi màu liên tục, lấp lánh ✨ ở các góc.",
            "虹色に移り変わる縁と、角にきらめく✨。",
            "A shifting rainbow frame with ✨ sparkles on the corners.",
        ],
        asset: "holo",
    },
];

/// Mọi món: CATALOG ở trên + các bộ Miu (miu_sets.rs). Dùng hàm này thay vì CATALOG.
pub fn items() -> impl Iterator<Item = &'static Item> {
    CATALOG.iter().chain(crate::miu_sets::SETS.iter().map(|s| &s.item))
}

/// Vị trí ngôn ngữ trong `Item::name` / `desc` (vi = 0, ja = 1, en = 2).
pub fn lang_index(lang: crate::persona::Lang) -> usize {
    match lang {
        crate::persona::Lang::Vi => 0,
        crate::persona::Lang::Ja => 1,
        crate::persona::Lang::En => 2,
    }
}

pub fn item(id: &str) -> Option<&'static Item> {
    items().find(|i| i.id == id)
}

/// Món mặc định (giá 0) của một nhóm chọn được.
pub fn default_of(cat: Category) -> &'static Item {
    items()
        .find(|i| i.cat == cat && i.price == 0)
        .expect("mỗi nhóm chọn được có một món mặc định (test catalog_is_sane)")
}

// ─── Trạng thái ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Collection {
    #[serde(default)]
    pub candy: u32,
    #[serde(default)]
    pub earned_total: u32,
    #[serde(default)]
    pub spent_total: u32,
    /// Id đã mua. Món giá 0 không lưu (luôn có).
    #[serde(default)]
    pub owned: Vec<String>,
    #[serde(default = "default_skin")]
    pub skin: String,
    #[serde(default = "default_side_set")]
    pub side_set: String,
    #[serde(default = "default_miu_set")]
    pub miu_set: String,
    #[serde(default = "default_bubble")]
    pub bubble: String,
    #[serde(default = "default_flower")]
    pub flower: String,
    #[serde(default = "default_widget_deco")]
    pub widget_deco: String,
    /// Hiệu ứng đã mua và đang bật.
    #[serde(default)]
    pub effects_on: Vec<String>,
    /// Gacha (gacha.rs): số lượt liền chưa ra Hiếm+ / chưa ra Sử thi, tổng lượt, lịch sử.
    #[serde(default)]
    pub pity: u32,
    #[serde(default)]
    pub epic_pity: u32,
    #[serde(default)]
    pub pulls: u32,
    #[serde(default)]
    pub history: Vec<crate::gacha::HistEntry>,
    /// Phiên bản bảng giá. File cũ (0) có kẹo theo giá ×1 → `migrate` nhân 10.
    #[serde(default)]
    pub econ: u32,
}

fn default_skin() -> String {
    "skin.lavender".into()
}
fn default_side_set() -> String {
    "side.bunny".into()
}
fn default_miu_set() -> String {
    "miu.classic".into()
}
fn default_bubble() -> String {
    "bubble.classic".into()
}
fn default_flower() -> String {
    "flower.sakura".into()
}
fn default_widget_deco() -> String {
    "wdeco.none".into()
}

impl Default for Collection {
    fn default() -> Self {
        Collection {
            candy: 0,
            earned_total: 0,
            spent_total: 0,
            owned: Vec::new(),
            skin: default_skin(),
            side_set: default_side_set(),
            miu_set: default_miu_set(),
            bubble: default_bubble(),
            flower: default_flower(),
            widget_deco: default_widget_deco(),
            effects_on: Vec::new(),
            pity: 0,
            epic_pity: 0,
            pulls: 0,
            history: Vec::new(),
            econ: ECON,
        }
    }
}

// ─── Kiếm kẹo ────────────────────────────────────────────────────────────────

/// Kẹo mỗi điểm thân thiết. Tất cả ×10 (bảng giá v2: giá ×10 để gacha 100 🍬/lượt
/// rẻ hơn đổi thẳng); muốn game cho nhiều hơn chỉ đổi một số.
pub fn candy_mult(ev: Event) -> u32 {
    match ev {
        Event::Present
        | Event::TaskDone
        | Event::FocusDone
        | Event::Chat
        | Event::Headpat
        | Event::Game
        | Event::Named => 10,
    }
}

/// Thưởng thêm chỉ bằng kẹo cho mỗi cột mốc thân thiết (quen N ngày, chuỗi, lên cấp, sinh nhật).
pub const MILESTONE_BONUS: u32 = 200;

/// Phiên bản bảng giá hiện tại. 2 = giá và kẹo ×10 (thêm gacha).
pub const ECON: u32 = 2;

/// File collection.json từ trước khi có gacha: kẹo theo giá cũ → nhân 10 để giữ
/// nguyên sức mua. Trả true nếu có đổi. Gọi mỗi lần đọc file (idempotent nhờ `econ`).
pub fn migrate(c: &mut Collection) -> bool {
    if c.econ >= ECON {
        return false;
    }
    c.candy = c.candy.saturating_mul(10);
    c.earned_total = c.earned_total.saturating_mul(10);
    c.spent_total = c.spent_total.saturating_mul(10);
    c.econ = ECON;
    true
}

/// Các sự kiện lặp lại hằng ngày (Named chỉ một lần trong đời, không tính vào trần ngày).
const DAILY_EVENTS: [Event; 6] =
    [Event::Present, Event::TaskDone, Event::FocusDone, Event::Chat, Event::Headpat, Event::Game];

/// Chỉ bản dev: mở khoá mọi món trong catalog (để thử giao diện, không trừ kẹo).
pub fn unlock_all(c: &mut Collection) {
    for it in items().filter(|i| i.price > 0) {
        if !c.owned.iter().any(|o| o == it.id) {
            c.owned.push(it.id.to_string());
        }
    }
}

/// Kẹo tối đa một ngày thường (35 với bảng hiện tại). Dùng để chặn giá vô lý.
pub fn max_daily_candy() -> u32 {
    DAILY_EVENTS.iter().map(|&ev| crate::bond::daily_cap(ev) * candy_mult(ev)).sum()
}

/// Cộng kẹo cho `gain` điểm thân thiết vừa nhận. Trả số kẹo thực cộng.
pub fn earn(c: &mut Collection, ev: Event, gain: u32) -> u32 {
    grant(c, gain * candy_mult(ev))
}

/// Cộng thẳng `amount` kẹo (cột mốc…). 0 thì không đổi gì.
pub fn grant(c: &mut Collection, amount: u32) -> u32 {
    if amount == 0 {
        return 0;
    }
    c.candy = c.candy.saturating_add(amount);
    c.earned_total = c.earned_total.saturating_add(amount);
    amount
}

/// Kẹo kiếm được hôm nay, suy từ bảng điểm hôm nay của bond.json.
pub fn earned_today(b: &crate::bond::Bond, today: crate::bond::Day) -> u32 {
    DAILY_EVENTS.iter().map(|&ev| crate::bond::earned_today(b, ev, today) * candy_mult(ev)).sum()
}

// ─── Mua / chọn ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuyError {
    NotFound,
    AlreadyOwned,
    /// Còn thiếu bao nhiêu kẹo.
    NotEnough { need: u32 },
}

impl BuyError {
    /// Mã cho trang Bộ sưu tập dịch ra 3 thứ tiếng.
    pub fn code(self) -> String {
        match self {
            BuyError::NotFound => "not_found".into(),
            BuyError::AlreadyOwned => "already_owned".into(),
            BuyError::NotEnough { need } => format!("not_enough:{need}"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectError {
    NotFound,
    WrongCategory,
    NotOwned,
}

impl SelectError {
    pub fn code(self) -> &'static str {
        match self {
            SelectError::NotFound => "not_found",
            SelectError::WrongCategory => "wrong_category",
            SelectError::NotOwned => "not_owned",
        }
    }
}

pub fn owns(c: &Collection, item: &Item) -> bool {
    item.price == 0 || c.owned.iter().any(|o| o == item.id)
}

/// Đổi kẹo lấy món `id`. Thất bại thì không đổi gì.
pub fn buy(c: &mut Collection, id: &str) -> Result<&'static Item, BuyError> {
    let it = item(id).ok_or(BuyError::NotFound)?;
    if owns(c, it) {
        return Err(BuyError::AlreadyOwned);
    }
    if c.candy < it.price {
        return Err(BuyError::NotEnough { need: it.price - c.candy });
    }
    c.candy -= it.price;
    c.spent_total = c.spent_total.saturating_add(it.price);
    c.owned.push(it.id.to_string());
    Ok(it)
}

/// Dùng món `id` cho nhóm `cat` (skin / bộ nhân vật / bộ mặt). Phải đã có.
pub fn select(c: &mut Collection, cat: Category, id: &str) -> Result<&'static Item, SelectError> {
    let it = item(id).ok_or(SelectError::NotFound)?;
    if it.cat != cat || !cat.selectable() {
        return Err(SelectError::WrongCategory);
    }
    if !owns(c, it) {
        return Err(SelectError::NotOwned);
    }
    let slot = match cat {
        Category::Skin => &mut c.skin,
        Category::SideSet => &mut c.side_set,
        Category::MiuSet => &mut c.miu_set,
        Category::Bubble => &mut c.bubble,
        Category::Flower => &mut c.flower,
        Category::WidgetDeco => &mut c.widget_deco,
        _ => unreachable!("selectable() đã lọc"),
    };
    *slot = it.id.to_string();
    Ok(it)
}

/// Bật / tắt một hiệu ứng đã mua.
pub fn set_effect(c: &mut Collection, id: &str, on: bool) -> Result<&'static Item, SelectError> {
    let it = item(id).ok_or(SelectError::NotFound)?;
    if it.cat != Category::Effect {
        return Err(SelectError::WrongCategory);
    }
    if !owns(c, it) {
        return Err(SelectError::NotOwned);
    }
    c.effects_on.retain(|e| e != it.id);
    if on {
        c.effects_on.push(it.id.to_string());
        c.effects_on.sort();
    }
    Ok(it)
}

/// Hiệu ứng loại `kind` (asset, vd. "fireworks") có đang bật không.
pub fn effect_on(c: &Collection, kind: &str) -> bool {
    c.effects_on
        .iter()
        .filter_map(|id| item(id))
        .any(|it| it.cat == Category::Effect && it.asset == kind && owns(c, it))
}

/// Món đang dùng của nhóm `cat`: id lưu trong file phải có thật và đã sở hữu,
/// không thì về mặc định (file bị sửa tay, món bị gỡ khỏi catalog…). Chỉ trả
/// `&'static` từ catalog — không bao giờ đẩy chuỗi từ JSON thẳng ra webview.
pub fn selected(c: &Collection, cat: Category) -> &'static Item {
    let id = match cat {
        Category::Skin => &c.skin,
        Category::SideSet => &c.side_set,
        Category::MiuSet => &c.miu_set,
        Category::Bubble => &c.bubble,
        Category::Flower => &c.flower,
        Category::WidgetDeco => &c.widget_deco,
        _ => return default_of(cat),
    };
    item(id).filter(|it| it.cat == cat && owns(c, it)).unwrap_or_else(|| default_of(cat))
}

pub fn effective_skin(c: &Collection) -> &'static str {
    selected(c, Category::Skin).asset
}

pub fn effective_miu_dir(c: &Collection) -> &'static str {
    selected(c, Category::MiuSet).asset
}

/// Đuôi ảnh của bộ mặt Miu ở thư mục `dir` (khai trong miu_sets.rs).
pub fn miu_ext(dir: &str) -> &'static str {
    crate::miu_sets::ext_of(dir)
}

pub fn effective_miu_ext(c: &Collection) -> &'static str {
    miu_ext(effective_miu_dir(c))
}

pub fn effective_bubble(c: &Collection) -> &'static str {
    selected(c, Category::Bubble).asset
}

pub fn effective_flower(c: &Collection) -> &'static str {
    selected(c, Category::Flower).asset
}

/// Trang trí của 3 widget ghim (asset trong widget-deco.css).
pub fn effective_widget_deco(c: &Collection) -> &'static str {
    selected(c, Category::WidgetDeco).asset
}

pub fn effective_side_set(c: &Collection) -> &'static str {
    selected(c, Category::SideSet).asset
}

/// Catalog + trạng thái cho trang Bộ sưu tập (JSON). `pet_model`: model đang dùng
/// (đã qua models::effective), để đánh dấu "Đang dùng" ở nhóm Model.
pub fn catalog_json(c: &Collection, lang_idx: usize, pet_model: &str) -> Value {
    let items: Vec<Value> = items()
        .map(|it| {
            json!({
                "id": it.id,
                "cat": it.cat.code(),
                "rarity": it.rarity.code(),
                "price": it.price,
                "name": it.name[lang_idx.min(2)],
                "desc": it.desc[lang_idx.min(2)],
                "asset": it.asset,
                // Ảnh đại diện bộ nhân vật (ảnh ở bảng Cài đặt), lấy từ sidegirl::SETS.
                "thumb": match it.cat {
                    Category::SideSet => crate::sidegirl::thumb(it.asset).to_string(),
                    Category::MiuSet => format!("{}/rps-cheer.{}", it.asset, miu_ext(it.asset)),
                    Category::Model => crate::models::preview_url(it.asset),
                    _ => String::new(),
                },
                "owned": owns(c, it),
                "selected": if it.cat == Category::Model {
                    it.asset == pet_model
                } else {
                    it.cat.selectable() && selected(c, it.cat).id == it.id
                },
                "on": it.cat == Category::Effect && c.effects_on.iter().any(|e| e == it.id),
            })
        })
        .collect();
    json!(items)
}

// ─── Phần Tauri: file, sự kiện, áp dụng ngay ─────────────────────────────────

mod glue {
    use super::*;
    use tauri::{AppHandle, Emitter, Manager};

    use crate::{persona, store};

    /// Toàn bộ dữ liệu cho trang Bộ sưu tập và các nơi hiện số kẹo.
    pub fn view(app: &AppHandle) -> Value {
        let s = store::load_settings(app);
        let a = persona::address(&s);
        let c = store::load_collection(app);
        let b = store::load_bond(app);
        let (today, _) = crate::companion::today_and_hour(crate::commands::now_millis() as u64 / 1000);
        let lang_idx = lang_index(a.lang);
        json!({
            "candy": c.candy,
            "earnedTotal": c.earned_total,
            "spentTotal": c.spent_total,
            "today": earned_today(&b, today),
            "todayCap": max_daily_candy(),
            "skin": effective_skin(&c),
            "sideSet": effective_side_set(&c),
            "miuDir": effective_miu_dir(&c),
            "miuExt": effective_miu_ext(&c),
            // Kiểu bong bóng thoại (cửa sổ pet) và biểu tượng khi bôi chữ (bông hoa).
            "bubble": effective_bubble(&c),
            "flower": effective_flower(&c),
            "items": with_model_status(
                app,
                catalog_json(&c, lang_idx, &crate::models::effective(&s.pet_model, &c, &s.custom_models, &crate::model_dl::installed_ids(app))),
            ),
            // 🎰 Candy Capsule: giá, tỉ lệ, bảo hiểm, lịch sử.
            "gacha": crate::gacha::view_json(&c, lang_idx),
            // Bunny Lounge đang tắt thì trang nhắc bật trong Cài đặt.
            "sideGirls": s.side_girls,
            // Bản dev (npm run dev): trang hiện nút thử +1000 kẹo / mở hết.
            "debug": cfg!(debug_assertions),
            "lang": a.lang.code(),
            "pet": a.pet,
            "you": a.you,
            "me": a.me,
        })
    }

    /// Thẻ model: đã tải chưa, đang tải bao nhiêu %, dung lượng, có bản mới không (model_dl.rs).
    fn with_model_status(app: &AppHandle, mut items: Value) -> Value {
        for it in items.as_array_mut().into_iter().flatten() {
            if it["cat"] != "model" {
                continue;
            }
            let st = crate::model_dl::status_json(app, it["asset"].as_str().unwrap_or(""));
            if let (Some(obj), Some(st)) = (it.as_object_mut(), st.as_object()) {
                obj.extend(st.clone());
            }
        }
        items
    }

    /// Kẹo / đồ vừa đổi: báo mọi cửa sổ và áp dụng ngay (widget đổi màu, nhân
    /// vật bám mép đổi ảnh). Không gì phải khởi động lại.
    pub fn changed(app: &AppHandle) {
        let _ = app.emit("collection-changed", view(app));
        crate::calendar_pin::broadcast(app);
        crate::clock_pin::broadcast(app);
        crate::weather_net::broadcast(app);
        let c = store::load_collection(app);
        crate::sidegirl::set_side_set(app, effective_side_set(&c));
    }

    /// Vừa cộng được `gain` điểm thân thiết cho `ev` → cộng kẹo tương ứng.
    pub fn earn_from(app: &AppHandle, ev: Event, gain: u32) {
        if gain == 0 {
            return;
        }
        match store::update_collection(app, |c| earn(c, ev, gain)) {
            Ok(got) if got > 0 => {
                let _ = app.emit("collection-changed", view(app));
            }
            Ok(_) => {}
            Err(e) => eprintln!("[collection] lưu kẹo lỗi: {e}"),
        }
    }

    /// Thưởng cột mốc (+MILESTONE_BONUS).
    pub fn grant_bonus(app: &AppHandle, amount: u32) {
        match store::update_collection(app, |c| grant(c, amount)) {
            Ok(got) if got > 0 => {
                let _ = app.emit("collection-changed", view(app));
            }
            Ok(_) => {}
            Err(e) => eprintln!("[collection] lưu kẹo lỗi: {e}"),
        }
    }

    /// Chạy hiệu ứng `kind` trên pet nếu đã mua, đang bật và pet đang hiện.
    pub fn fx(app: &AppHandle, kind: &str) {
        if !effect_on(&store::load_collection(app), kind) {
            return;
        }
        preview_fx(app, kind);
    }

    /// Chạy hiệu ứng không cần sở hữu (nút "Xem thử" trong Bộ sưu tập).
    pub fn preview_fx(app: &AppHandle, kind: &str) {
        let Some(w) = app.get_webview_window("main") else { return };
        if !w.is_visible().unwrap_or(false) {
            return;
        }
        let _ = app.emit_to("main", "pet-message", json!({ "command": "fx", "kind": kind }));
    }
}

pub use glue::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_sane() {
        let mut ids: Vec<&str> = items().map(|i| i.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), items().count(), "id trùng");
        for cat in [Category::Skin, Category::SideSet, Category::MiuSet, Category::Bubble, Category::Flower, Category::WidgetDeco] {
            let free = items().filter(|i| i.cat == cat && i.price == 0).count();
            assert_eq!(free, 1, "{cat:?} phải có đúng một món mặc định");
        }
        for it in items() {
            assert!(it.price == 0 || it.rarity.prices().contains(&it.price), "{}: giá không khớp hạng", it.id);
            assert_eq!(it.cat == Category::Model, it.rarity == Rarity::Legendary, "{}: Huyền thoại chỉ dành cho model", it.id);
            assert!(it.price <= 30 * max_daily_candy(), "{}: quá 30 ngày kẹo", it.id);
            assert!(it.name.iter().all(|n| !n.is_empty()) && it.desc.iter().all(|d| !d.is_empty()), "{}", it.id);
            assert!(it.id.starts_with(match it.cat {
                Category::Skin => "skin.",
                Category::SideSet => "side.",
                Category::MiuSet => "miu.",
                Category::Effect => "fx.",
                Category::LinePack => "lines.",
                Category::Bubble => "bubble.",
                Category::Flower => "flower.",
                Category::Model => "model.",
                Category::WidgetDeco => "wdeco.",
            }));
            assert_eq!(Category::from_code(it.cat.code()), Some(it.cat));
        }
    }

    #[test]
    fn every_widget_deco_exists_in_css() {
        let css = include_str!("../../app/widget-deco.css");
        for it in items().filter(|i| i.cat == Category::WidgetDeco && i.price > 0) {
            assert!(css.contains(&format!("[data-deco=\"{}\"]", it.asset)), "{}: thiếu trong widget-deco.css", it.id);
        }
        let mut c = Collection::default();
        assert_eq!(effective_widget_deco(&c), "none");
        c.widget_deco = "wdeco.holo".into();
        assert_eq!(effective_widget_deco(&c), "none", "chưa sở hữu thì về mặc định");
        c.owned.push("wdeco.holo".into());
        assert_eq!(effective_widget_deco(&c), "holo");
    }

    #[test]
    fn every_skin_exists_in_css_and_every_miu_set_has_all_faces() {
        let css = include_str!("../../app/skins.css");
        for it in items().filter(|i| i.cat == Category::Skin) {
            let sel = format!("[data-skin=\"{}\"]", it.asset);
            assert!(css.contains(&sel), "skins.css thiếu {sel}");
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../app");
        for it in items().filter(|i| i.cat == Category::MiuSet) {
            for face in ["wink", "cheer", "think", "draw", "pout", "sleepy", "surprised", "shy", "fight"] {
                let p = root.join(it.asset).join(format!("rps-{face}.{}", miu_ext(it.asset)));
                assert!(p.is_file(), "{}: thiếu {}", it.id, p.display());
            }
        }
    }

    #[test]
    fn candy_follows_bond_points() {
        assert_eq!(max_daily_candy(), 350);
        let mut c = Collection::default();
        assert_eq!(earn(&mut c, Event::TaskDone, 2), 20);
        assert_eq!(earn(&mut c, Event::Present, 0), 0);
        assert_eq!(grant(&mut c, MILESTONE_BONUS), 200);
        assert_eq!((c.candy, c.earned_total), (220, 220));
        let before = c.clone();
        earn(&mut c, Event::Game, 0);
        assert_eq!(c, before, "gain 0 không đổi gì");
    }

    #[test]
    fn earned_today_sums_bond_ledger() {
        use crate::bond::{award, Bond};
        let mut b = Bond::default();
        award(&mut b, Event::TaskDone, 100);
        award(&mut b, Event::Present, 100);
        award(&mut b, Event::Named, 100);
        assert_eq!(earned_today(&b, 100), 70, "Named không tính vào kẹo ngày");
        assert_eq!(earned_today(&b, 101), 0);
    }

    #[test]
    fn buying() {
        let mut c = Collection::default();
        assert_eq!(buy(&mut c, "skin.sakura"), Err(BuyError::NotEnough { need: 600 }));
        assert_eq!(c, Collection::default(), "thất bại không đổi gì");
        c.candy = 1000;
        assert_eq!(buy(&mut c, "skin.sakura").map(|i| i.id), Ok("skin.sakura"));
        assert_eq!((c.candy, c.spent_total), (400, 600));
        assert_eq!(buy(&mut c, "skin.sakura"), Err(BuyError::AlreadyOwned));
        assert_eq!(buy(&mut c, "skin.lavender"), Err(BuyError::AlreadyOwned), "món mặc định luôn có");
        assert_eq!(buy(&mut c, "skin.nope"), Err(BuyError::NotFound));
        assert_eq!(BuyError::NotEnough { need: 7 }.code(), "not_enough:7");
    }

    #[test]
    fn selecting_and_effects() {
        let mut c = Collection::default();
        assert_eq!(select(&mut c, Category::Skin, "skin.sakura"), Err(SelectError::NotOwned));
        assert_eq!(select(&mut c, Category::Skin, "fx.confetti"), Err(SelectError::WrongCategory));
        assert_eq!(select(&mut c, Category::Effect, "fx.confetti"), Err(SelectError::WrongCategory));
        assert_eq!(select(&mut c, Category::Skin, "x"), Err(SelectError::NotFound));
        c.candy = 5000;
        buy(&mut c, "skin.sakura").unwrap();
        assert!(select(&mut c, Category::Skin, "skin.sakura").is_ok());
        assert_eq!(effective_skin(&c), "sakura");
        assert!(select(&mut c, Category::Skin, "skin.lavender").is_ok(), "về mặc định luôn được");
        assert_eq!(effective_skin(&c), "lavender");

        assert_eq!(set_effect(&mut c, "fx.confetti", true), Err(SelectError::NotOwned));
        buy(&mut c, "fx.confetti").unwrap();
        assert!(set_effect(&mut c, "fx.confetti", true).is_ok());
        assert!(set_effect(&mut c, "fx.confetti", true).is_ok(), "bật hai lần không nhân đôi");
        assert_eq!(c.effects_on, vec!["fx.confetti".to_string()]);
        assert!(effect_on(&c, "confetti"));
        assert!(!effect_on(&c, "fireworks"));
        assert!(set_effect(&mut c, "fx.confetti", false).is_ok());
        assert!(!effect_on(&c, "confetti"));
    }

    #[test]
    fn miu_sets_pick_their_image_type() {
        assert_eq!(miu_ext("media/game"), "png");
        assert_eq!(miu_ext("media/miu/snow"), "webp");
        let mut c = Collection::default();
        assert_eq!(effective_miu_ext(&c), "png");
        c.candy = 1500;
        buy(&mut c, "miu.snow").unwrap();
        select(&mut c, Category::MiuSet, "miu.snow").unwrap();
        assert_eq!((effective_miu_dir(&c), effective_miu_ext(&c)), ("media/miu/snow", "webp"));
    }

    #[test]
    fn unlock_all_owns_everything_once() {
        let mut c = Collection::default();
        unlock_all(&mut c);
        unlock_all(&mut c);
        assert_eq!(c.owned.len(), items().filter(|i| i.price > 0).count());
        assert!(items().all(|i| owns(&c, i)));
        assert_eq!(c.candy, 0, "không cộng kẹo");
    }

    #[test]
    fn garbage_in_file_falls_back_to_defaults() {
        let mut c = Collection::default();
        c.skin = "skin.sakura".into(); // chưa mua
        c.side_set = "side.nope".into();
        c.miu_set = "fx.confetti".into(); // sai nhóm
        c.effects_on = vec!["fx.fireworks".into()]; // chưa mua
        assert_eq!(effective_skin(&c), "lavender");
        assert_eq!(effective_side_set(&c), "bunny");
        assert_eq!(effective_miu_dir(&c), "media/game");
        assert!(!effect_on(&c, "fireworks"));
        let mut parsed: Collection = serde_json::from_str("{}").unwrap();
        assert_eq!(parsed.econ, 0, "file cũ chưa có bảng giá");
        migrate(&mut parsed);
        assert_eq!(parsed, Collection::default());
    }

    #[test]
    fn old_files_keep_their_buying_power() {
        let mut c: Collection = serde_json::from_str(r#"{"candy":120,"earned_total":300,"spent_total":180,"owned":["skin.sakura"]}"#).unwrap();
        assert!(migrate(&mut c));
        assert_eq!((c.candy, c.earned_total, c.spent_total, c.econ), (1200, 3000, 1800, ECON));
        assert!(!migrate(&mut c), "chỉ nhân một lần");
        assert_eq!(c.candy, 1200);
        assert!(!migrate(&mut Collection::default()), "người mới không bị nhân");
    }

    #[test]
    fn catalog_json_marks_state() {
        let mut c = Collection::default();
        c.candy = 600;
        buy(&mut c, "skin.sakura").unwrap();
        let v = catalog_json(&c, 2, "hiyori");
        let items = v.as_array().unwrap();
        assert_eq!(items.len(), super::items().count());
        let sakura = items.iter().find(|i| i["id"] == "skin.sakura").unwrap();
        assert_eq!((sakura["owned"].as_bool(), sakura["selected"].as_bool()), (Some(true), Some(false)));
        assert_eq!(sakura["name"], "Sakura");
        let lav = items.iter().find(|i| i["id"] == "skin.lavender").unwrap();
        assert_eq!(lav["selected"], true);
        let fx = items.iter().find(|i| i["id"] == "fx.confetti").unwrap();
        assert_eq!((&fx["owned"], &fx["on"]), (&json!(false), &json!(false)));
        let model = |v: &Value| v.as_array().unwrap().iter().find(|i| i["id"] == "model.gothic-aristocrat").unwrap()["selected"].clone();
        assert_eq!(model(&v), json!(false));
        assert_eq!(model(&catalog_json(&c, 2, "gothic-aristocrat")), json!(true));
        let thumb = v.as_array().unwrap().iter().find(|i| i["id"] == "model.gothic-aristocrat").unwrap()["thumb"].clone();
        assert_eq!(thumb, json!("media/model-previews/gothic-aristocrat.png"));
    }
}
