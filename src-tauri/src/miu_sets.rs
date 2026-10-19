// Các bộ mặt Miu (Bộ sưu tập → tab Miu mini). Một bộ = 9 ảnh cùng tên trong một
// thư mục, dùng cho widget ghim, thẻ dự báo, trang Mini Games và mọi cửa sổ game
// (js/miu-skin.js). Thư mục + đuôi ảnh đi theo bộ, nên bộ nào cũng có thể PNG
// hay WebP.
//
// ── Thêm một bộ Miu mới ──────────────────────────────────────────────────────
// 1. Thả 9 ảnh nền trong suốt vào  app/media/miu/<tên-bộ>/rps-<mặt>.webp, đủ 9
//    mặt trong FACES (thiếu mặt nào thì chép một ảnh gần nghĩa sang tên đó).
//    Cao ~300 px (widget) hoặc ~520 px (nếu muốn nét cả trong game).
// 2. Thêm một mục vào SETS bên dưới (id "miu.<tên-bộ>", tên vi/ja/en, giá).
// 3. cargo test: test kiểm đủ 9 ảnh, id không trùng, giá khớp hạng.
// Không phải sửa chỗ nào khác: catalog, trang Bộ sưu tập, widget, game tự nhận.

use crate::collection::{Category, Item, Rarity};

/// 9 mặt app dùng (tên file rps-<mặt>.<ext>). Danh sách tham chiếu khi thêm bộ; test kiểm đủ.
#[cfg_attr(not(test), allow(dead_code))]
pub const FACES: [&str; 9] = ["wink", "cheer", "think", "draw", "pout", "sleepy", "surprised", "shy", "fight"];

pub struct MiuSet {
    /// Món trong catalog; `asset` = thư mục ảnh (tính từ app/).
    pub item: Item,
    /// Đuôi ảnh: "png" | "webp".
    pub ext: &'static str,
}

pub const SETS: &[MiuSet] = &[
    // Bộ gốc, có sẵn (giá 0). Ảnh ở media/game vì game dùng chung từ trước.
    MiuSet {
        item: Item {
            id: "miu.classic",
            cat: Category::MiuSet,
            rarity: Rarity::Rare,
            price: 0,
            name: ["Miu áo hồng", "ピンクのミウ", "Classic Miu"],
            desc: ["Bộ biểu cảm quen thuộc.", "いつもの表情セット。", "The familiar set of faces."],
            asset: "media/game",
        },
        ext: "png",
    },
    MiuSet {
        item: Item {
            id: "miu.snow",
            cat: Category::MiuSet,
            rarity: Rarity::Rare,
            price: 150,
            name: ["Miu tuyết", "雪のミウ", "Snow Miu"],
            desc: [
                "Miu tai mèo áo len trắng, 9 biểu cảm mùa đông.",
                "白いニットの猫耳ミウ、冬の表情9種。",
                "Cat-eared Miu in a white knit coat, nine winter faces.",
            ],
            asset: "media/miu/snow",
        },
        ext: "webp",
    },
    MiuSet {
        item: Item {
            id: "miu.gothic",
            cat: Category::MiuSet,
            rarity: Rarity::Rare,
            price: 150,
            name: ["Miu Gothic", "ゴシックのミウ", "Gothic Miu"],
            desc: [
                "Miu tai mèo váy gothic lolita tím đen, 9 biểu cảm.",
                "紫と黒のゴシックロリータの猫耳ミウ、表情9種。",
                "Cat-eared Miu in a purple-and-black gothic lolita dress, nine faces.",
            ],
            asset: "media/miu/gothic",
        },
        ext: "webp",
    },
];

/// Đuôi ảnh của bộ có thư mục `dir`; thư mục lạ → "png" (bộ gốc).
pub fn ext_of(dir: &str) -> &'static str {
    SETS.iter().find(|s| s.item.asset == dir).map_or("png", |s| s.ext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_set_has_all_faces_and_a_clean_id() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../app");
        for s in SETS {
            assert!(s.item.id.starts_with("miu.") && s.item.cat == Category::MiuSet, "{}", s.item.id);
            assert!(matches!(s.ext, "png" | "webp"), "{}: đuôi lạ {}", s.item.id, s.ext);
            for face in FACES {
                let p = root.join(s.item.asset).join(format!("rps-{face}.{}", s.ext));
                assert!(p.is_file(), "{}: thiếu {}", s.item.id, p.display());
            }
        }
        assert_eq!(SETS.iter().filter(|s| s.item.price == 0).count(), 1, "đúng một bộ có sẵn");
        assert_eq!(ext_of("media/miu/snow"), "webp");
        assert_eq!(ext_of("media/game"), "png");
        assert_eq!(ext_of("lạ"), "png");
    }
}
