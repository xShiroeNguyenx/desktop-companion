// Trang 🍬 Bộ sưu tập (collection.html): chữ vi/ja/en, gom món theo nhóm, sắp
// xếp, trạng thái nút (Đổi / Dùng / Đang dùng / Bật), dịch mã lỗi của Rust,
// ô xem trước. Dữ liệu món do Rust gửi (collection::view). Hàm thuần, không
// đụng DOM; dùng trong cửa sổ (window.CollectionCore) và node --test.
(function (root) {
  /** Thứ tự tab. Nhóm không có món thì không hiện. */
  const CATS = ['skin', 'effect', 'side_set', 'miu_set', 'bubble', 'flower', 'line_pack'];
  /** Icon trên tab. */
  const TAB_ICONS = { skin: '🎨', effect: '✨', side_set: '🖼️', miu_set: '🐰', bubble: '💬', flower: '🌸', line_pack: '📜' };
  const SELECTABLE = ['skin', 'side_set', 'miu_set', 'bubble', 'flower'];

  const T = {
    vi: {
      title: 'Bộ sưu tập', subtitle: 'Thu thập và mở khoá những điều dễ thương cùng {pet}! 💗',
      candyLabel: 'Kẹo hiện tại', candy: 'kẹo', todayLabel: 'Hôm nay đã nhận', today: '{n} / {cap}',
      bubble: '{Pet} thích những chủ đề xinh xắn lắm đó! Cùng thu thập thêm nhé! 💗',
      howTitle: 'Cách nhận kẹo', earn: ['✅ Hoàn thành việc', '🍅 Xong phiên Tập trung', '🎮 Thắng mini game', '💗 Chat & vuốt đầu', '🏆 Đạt cột mốc'],
      noMoney: 'Không thể mua kẹo bằng tiền thật.',
      howTo: 'Kẹo đến từ việc đã xong, phiên Tập trung, chat, vuốt đầu và thắng game (có trần mỗi ngày). Mỗi cột mốc thân thiết +200 🍬. Không bán, không mua bằng tiền thật.',
      tabs: { skin: 'Skin widget', effect: 'Hiệu ứng', side_set: 'Nhân vật', miu_set: 'Miu mini', bubble: 'Bong bóng', flower: 'Bông hoa', line_pack: 'Câu thoại' },
      rarity: { common: 'Thường', rare: 'Hiếm', epic: 'Sử thi' },
      using: 'Đang dùng', use: 'Dùng', buy: 'Đổi {price} 🍬', short: '🍬 Thiếu {n}', on: 'Đang bật', off: 'Bật',
      preview: 'Xem thử', free: 'Có sẵn', empty: 'Chưa có món nào ở nhóm này.', close: 'Đóng',
      bought: '{item} là của {you} rồi! 🍬', sideOff: 'Bunny Lounge đang tắt: bật trong Cài đặt → 🎨 Diện mạo để thấy nhân vật.',
      modes: { gacha: 'Candy Capsule', shop: 'Cửa hàng kẹo', history: 'Lịch sử' },
      g: {
        bubble: 'Hôm nay thử vận may nào? Cùng quay thật nhiều đồ xinh nhé! 💗',
        sign: 'Sweet Dreams', signSub: 'Skin, nhân vật, bộ mặt Miu và hiệu ứng, quay là có!',
        pull1: 'Quay 1 lần', pull10: 'Quay 10 lần', short: 'Thiếu {n} 🍬',
        ratesTitle: 'Tỷ lệ nhận vật phẩm',
        rateDesc: { common: 'Vật phẩm phổ biến', rare: 'Vật phẩm hiếm', epic: 'Vật phẩm rất hiếm' },
        pityRare: 'Còn {n} lượt chắc chắn ra Hiếm+', pityEpic: 'Còn {n} lượt chắc chắn ra Sử thi',
        dupNote: 'Trùng món đã có: hoàn {c} / {r} / {e} 🍬',
        featured: 'Vật phẩm nổi bật trong máy kẹo', seeAll: 'Xem cửa hàng →',
        resultTitle: 'Kết quả quay', newTag: 'MỚI!', dupTag: 'Trùng · +{n} 🍬', pityTag: 'Bảo hiểm',
        sumNew: 'Mới {n} món', sumRefund: 'Hoàn {c} 🍬', again: 'Quay tiếp ({c} 🍬)', ok: 'Nhận',
        locked: 'Máy kẹo mở khi Bộ sưu tập có đủ {n} món.',
        histTitle: 'Lịch sử quay', histTotal: 'Tổng {n} lượt', histEmpty: 'Chưa quay lượt nào. Thử vận may đi!',
        histNew: 'Món mới', histDup: 'Trùng',
        rvCommon: 'Mở nào~!', rvRare: '✦ HIẾM! ✦', rvEpic: '🌟 SỬ THI! 🌟', rvSkip: 'Bấm để bỏ qua',
      },
      err: {
        not_found: 'Không thấy món này.', already_owned: 'Đã có rồi mà~', not_enough: 'Còn thiếu {n} 🍬 nữa.',
        wrong_category: 'Món này không dùng ở đây được.', not_owned: 'Chưa có món này.', other: 'Không làm được: {e}',
      },
    },
    ja: {
      title: 'コレクション', subtitle: '{pet}といっしょに、かわいいものを集めて解放しよう！💗',
      candyLabel: '今のキャンディ', candy: '個', todayLabel: '今日もらった数', today: '{n} / {cap}',
      bubble: 'かわいいテーマ、大好き！いっしょに集めようね！💗',
      howTitle: 'キャンディのもらい方', earn: ['✅ タスク完了', '🍅 集中セッション', '🎮 ミニゲーム勝利', '💗 チャット・なでなで', '🏆 節目の達成'],
      noMoney: 'キャンディはお金では買えません。',
      howTo: 'キャンディはタスク完了・集中セッション・チャット・なでなで・ゲーム勝利でもらえます（1日の上限あり）。なかよし度の節目ごとに +200 🍬。販売も課金もありません。',
      tabs: { skin: 'ウィジェットスキン', effect: 'エフェクト', side_set: 'キャラクター', miu_set: 'ミニミウ', bubble: '吹き出し', flower: 'お花', line_pack: 'セリフ' },
      rarity: { common: 'ノーマル', rare: 'レア', epic: 'エピック' },
      using: '使用中', use: '使う', buy: '{price} 🍬 で交換', short: 'あと {n} 🍬', on: 'オン', off: 'オンにする',
      preview: 'プレビュー', free: '最初から', empty: 'このグループにはまだアイテムがありません。', close: '閉じる',
      bought: '{item} は{you}のもの！🍬', sideOff: 'バニーラウンジはオフです：設定 → 🎨 Diện mạo でオンにするとキャラクターが見えます。',
      modes: { gacha: 'キャンディカプセル', shop: 'キャンディショップ', history: '履歴' },
      g: {
        bubble: '今日の運だめし、してみる？かわいいもの、いっぱい出るといいね！💗',
        sign: 'Sweet Dreams', signSub: 'スキン・キャラ・ミウの表情・エフェクトが出るよ！',
        pull1: '1回まわす', pull10: '10回まわす', short: 'あと {n} 🍬',
        ratesTitle: '排出率',
        rateDesc: { common: 'よく出るアイテム', rare: 'めずらしいアイテム', epic: 'とてもめずらしいアイテム' },
        pityRare: 'あと {n} 回でレア以上確定', pityEpic: 'あと {n} 回でエピック確定',
        dupNote: 'ダブりは {c} / {r} / {e} 🍬 返ってくるよ',
        featured: 'カプセルの注目アイテム', seeAll: 'ショップを見る →',
        resultTitle: '結果', newTag: 'NEW!', dupTag: 'ダブり · +{n} 🍬', pityTag: '天井',
        sumNew: '新しいアイテム {n} 個', sumRefund: '{c} 🍬 返却', again: 'もう一度 ({c} 🍬)', ok: 'OK',
        locked: 'コレクションのアイテムが {n} 個になるとカプセルが使えます。',
        histTitle: 'ガチャ履歴', histTotal: '合計 {n} 回', histEmpty: 'まだ一度もまわしていません。運だめししてみよう！',
        histNew: '新規', histDup: 'ダブり',
        rvCommon: 'オープン～！', rvRare: '✦ レア！ ✦', rvEpic: '🌟 エピック！ 🌟', rvSkip: 'タップでスキップ',
      },
      err: {
        not_found: 'アイテムが見つかりません。', already_owned: 'もう持ってるよ～', not_enough: 'あと {n} 🍬 足りないよ。',
        wrong_category: 'ここでは使えないアイテムです。', not_owned: 'まだ持っていません。', other: 'できませんでした: {e}',
      },
    },
    en: {
      title: 'Collection', subtitle: 'Collect and unlock cute things with {pet}! 💗',
      candyLabel: 'Candy now', candy: 'candy', todayLabel: 'Earned today', today: '{n} / {cap}',
      bubble: "I love pretty themes! Let's collect more together! 💗",
      howTitle: 'How to earn candy', earn: ['✅ Finish tasks', '🍅 Focus sessions', '🎮 Win mini games', '💗 Chat & headpats', '🏆 Reach milestones'],
      noMoney: 'Candy can never be bought with real money.',
      howTo: 'Candy comes from finished tasks, focus sessions, chat, headpats and game wins (daily caps apply). Every friendship milestone adds +200 🍬. Never sold, never bought with real money.',
      tabs: { skin: 'Widget skins', effect: 'Effects', side_set: 'Characters', miu_set: 'Mini Miu', bubble: 'Bubbles', flower: 'Flower', line_pack: 'Lines' },
      rarity: { common: 'Common', rare: 'Rare', epic: 'Epic' },
      using: 'In use', use: 'Use', buy: 'Trade {price} 🍬', short: '{n} 🍬 short', on: 'On', off: 'Turn on',
      preview: 'Preview', free: 'Included', empty: 'Nothing in this group yet.', close: 'Close',
      bought: '{item} is yours now! 🍬', sideOff: 'Bunny Lounge is off: turn it on in Settings → 🎨 Diện mạo to see the characters.',
      modes: { gacha: 'Candy Capsule', shop: 'Candy shop', history: 'History' },
      g: {
        bubble: "Feeling lucky today? Let's spin for lots of cute things! 💗",
        sign: 'Sweet Dreams', signSub: 'Skins, characters, Miu faces and effects inside!',
        pull1: 'Spin once', pull10: 'Spin 10 times', short: '{n} 🍬 short',
        ratesTitle: 'Drop rates',
        rateDesc: { common: 'Common items', rare: 'Rare items', epic: 'Very rare items' },
        pityRare: '{n} more spins to a sure Rare+', pityEpic: '{n} more spins to a sure Epic',
        dupNote: 'Duplicates refund {c} / {r} / {e} 🍬',
        featured: 'Featured in the capsule machine', seeAll: 'Open the shop →',
        resultTitle: 'Your capsules', newTag: 'NEW!', dupTag: 'Duplicate · +{n} 🍬', pityTag: 'Pity',
        sumNew: '{n} new', sumRefund: '{c} 🍬 refunded', again: 'Spin again ({c} 🍬)', ok: 'Collect',
        locked: 'The capsule machine opens once the Collection has {n} items.',
        histTitle: 'Spin history', histTotal: '{n} spins in total', histEmpty: 'No spins yet. Try your luck!',
        histNew: 'New', histDup: 'Duplicate',
        rvCommon: 'Open up~!', rvRare: '✦ RARE! ✦', rvEpic: '🌟 EPIC! 🌟', rvSkip: 'Click to skip',
      },
      err: {
        not_found: 'No such item.', already_owned: 'You already have it~', not_enough: 'Still {n} 🍬 short.',
        wrong_category: 'This item cannot be used here.', not_owned: "You don't have this item yet.", other: 'Could not do that: {e}',
      },
    },
  };

  function strings(lang) {
    return T[lang] || T.vi;
  }

  function fill(s, vars) {
    return String(s).replace(/\{(\w+)\}/g, (m, k) => (vars && k in vars ? vars[k] : m));
  }

  /** Gom theo nhóm, giữ thứ tự CATS, bỏ nhóm trống. @returns {[string, object[]][]} */
  function groupByCategory(items) {
    return CATS.map((cat) => [cat, (items || []).filter((i) => i.cat === cat)]).filter(([, list]) => list.length);
  }

  /** Đang dùng / đang bật lên đầu, rồi món đã có, rồi rẻ trước. Không đổi mảng vào. */
  function sortItems(items) {
    const rank = (i) => (i.selected || i.on ? 0 : i.owned ? 1 : 2);
    return [...(items || [])].sort((a, b) => rank(a) - rank(b) || a.price - b.price || String(a.id).localeCompare(String(b.id)));
  }

  /**
   * Nút chính cho một món.
   * @returns {{kind: 'using'|'use'|'buy'|'short'|'on'|'off', label: string, disabled: boolean}}
   */
  function buttonState(item, candy, lang) {
    const S = strings(lang);
    if (!item.owned) {
      const need = item.price - (Number(candy) || 0);
      return need > 0
        ? { kind: 'short', label: fill(S.short, { n: need }), disabled: true }
        : { kind: 'buy', label: fill(S.buy, { price: item.price }), disabled: false };
    }
    if (item.cat === 'effect') {
      return item.on ? { kind: 'on', label: S.on, disabled: false } : { kind: 'off', label: S.off, disabled: false };
    }
    if (SELECTABLE.includes(item.cat)) {
      return item.selected ? { kind: 'using', label: S.using, disabled: true } : { kind: 'use', label: S.use, disabled: false };
    }
    return { kind: 'using', label: S.free, disabled: true };
  }

  function rarityClass(r) {
    return ['common', 'rare', 'epic'].includes(r) ? 'r-' + r : 'r-common';
  }

  /** Mã lỗi Rust ("not_enough:12", "not_owned", …) → câu cho người đọc. */
  function errorText(code, lang) {
    const S = strings(lang).err;
    const [key, arg] = String(code || '').split(':');
    if (key === 'not_enough') return fill(S.not_enough, { n: arg || '?' });
    if (S[key]) return S[key];
    return fill(S.other, { e: code });
  }

  /** Gradient xem trước của skin (asset trong collection.rs). ĐỒNG BỘ với app/skins.css. */
  const SWATCH = {
    matcha: ['#d6e8c4', '#b2d2a0'],
    cotton: ['#ffcee6', '#bee2ff'],
    latte: ['#ecd6be', '#cca884'],
    neon: ['#100c1e', '#1e0e2c'],
    lavender: ['#382c54', '#54325c'],
    sakura: ['#ffdae8', '#ffbad8'],
    night: ['#161a42', '#2c1e5a'],
    mint: ['#d0f4e6', '#b0e8d4'],
    sunset: ['#ffa676', '#c6568c'],
    ocean: ['#18487a', '#126c96'],
  };
  function swatch(asset) {
    const [a, b] = SWATCH[asset] || SWATCH.lavender;
    return `linear-gradient(160deg, ${a}, ${b})`;
  }
  /** Chữ trên ô xem trước skin sáng thì tối. */
  function swatchInk(asset) {
    return ['sakura', 'mint', 'matcha', 'cotton', 'latte'].includes(asset) ? '#5b2f47' : '#fff';
  }

  /** Màu nhấn (ô hôm nay) của skin. ĐỒNG BỘ với --w-accent trong app/skins.css. */
  const ACCENT = { matcha: '#6aa84f', cotton: '#ff6fb2', latte: '#b0703c', neon: '#ff3fa4', lavender: '#ff73ab', sakura: '#ff5fa2', night: '#8da2ff', mint: '#2fb58a', sunset: '#ffcf5a', ocean: '#4fd1ff' };
  function swatchAccent(asset) {
    return ACCENT[asset] || ACCENT.lavender;
  }

  /** Ảnh đại diện của một món có ảnh. */
  function thumb(item) {
    // Bộ nhân vật: Rust gửi sẵn đường dẫn (sidegirl::thumb), không đoán theo định dạng file.
    if (item.cat === 'side_set') return item.thumb || (item.asset === 'bunny' ? 'media/side/settings-girl.png' : '');
    if (item.cat === 'miu_set') return item.thumb || `${item.asset}/rps-cheer.png`;
    return '';
  }

  const EFFECT_ICON = { fireworks: '🎆', confetti: '🎊', petals: '🌸', hearts: '💕', bubbles: '🫧', shooting: '🌠' };
  /** Biểu tượng của món Bông hoa. ĐỒNG BỘ với js/deco-apply.js. */
  const FLOWER_GLYPH = { sakura: '🌸', star: '⭐', clover: '🍀', paw: '🐾', snow: '❄️' };
  function flowerGlyph(asset) {
    return FLOWER_GLYPH[asset] || FLOWER_GLYPH.sakura;
  }
  function effectIcon(asset) {
    return EFFECT_ICON[asset] || '✨';
  }

  /** Thứ tự chế độ trên thanh trên cùng. */
  const MODES = ['gacha', 'shop', 'history'];
  const RARITY_RANK = { common: 0, rare: 1, epic: 2 };

  /** Nút quay: đủ kẹo thì bấm được, thiếu thì báo thiếu bao nhiêu; máy khoá thì tắt. */
  function pullState(cost, candy, enabled, lang) {
    const g = strings(lang).g;
    if (!enabled) return { ok: false, label: '🔒' };
    const need = (Number(cost) || 0) - (Number(candy) || 0);
    return need > 0 ? { ok: false, label: fill(g.short, { n: need }) } : { ok: true, label: '' };
  }

  /** Tóm tắt một lần quay: số món mới, tổng kẹo hoàn, hạng cao nhất. */
  function summarize(results) {
    const list = results || [];
    let best = 'common';
    for (const r of list) if ((RARITY_RANK[r.rarity] || 0) > RARITY_RANK[best]) best = r.rarity;
    return {
      fresh: list.filter((r) => !r.dup).length,
      refund: list.reduce((s, r) => s + (Number(r.refund) || 0), 0),
      best,
    };
  }

  /** Món nổi bật dưới máy kẹo: chưa có trước, hiếm trước, món có ảnh trước. Tối đa `n`. */
  function featured(items, n) {
    const hasPic = (i) => (i.cat === 'side_set' || i.cat === 'miu_set' ? 0 : 1);
    return (items || [])
      .filter((i) => i.price > 0)
      .sort((a, b) => Number(a.owned) - Number(b.owned)
        || (RARITY_RANK[b.rarity] || 0) - (RARITY_RANK[a.rarity] || 0)
        || hasPic(a) - hasPic(b)
        || String(a.id).localeCompare(String(b.id)))
      .slice(0, n || 6);
  }

  /** Thời điểm trong lịch sử: "hh:mm dd/mm" (vi/en) hoặc "mm/dd hh:mm" (ja), giờ máy. */
  function formatTime(ms, lang) {
    const d = new Date(Number(ms) || 0);
    const p = (x) => String(x).padStart(2, '0');
    const hm = p(d.getHours()) + ':' + p(d.getMinutes());
    return lang === 'ja' ? `${p(d.getMonth() + 1)}/${p(d.getDate())} ${hm}` : `${hm} ${p(d.getDate())}/${p(d.getMonth() + 1)}`;
  }

  const api = { MODES, pullState, summarize, featured, formatTime, CATS, TAB_ICONS, SELECTABLE, T, strings, fill, groupByCategory, sortItems, buttonState, rarityClass, errorText, swatch, swatchInk, swatchAccent, flowerGlyph, thumb, effectIcon };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.CollectionCore = api;
})(typeof window !== 'undefined' ? window : globalThis);
