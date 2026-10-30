// Trang 🍬 Bộ sưu tập (collection.html): chữ vi/ja/en, gom món theo nhóm, sắp
// xếp, trạng thái nút (Đổi / Dùng / Đang dùng / Bật), dịch mã lỗi của Rust,
// ô xem trước. Dữ liệu món do Rust gửi (collection::view). Hàm thuần, không
// đụng DOM; dùng trong cửa sổ (window.CollectionCore) và node --test.
(function (root) {
  /** Thứ tự tab. Nhóm không có món thì không hiện. */
  const CATS = ['skin', 'widget_deco', 'effect', 'side_set', 'model', 'miu_set', 'bubble', 'flower', 'line_pack'];
  /** Icon trên tab. */
  const TAB_ICONS = { skin: '🎨', widget_deco: '🎀', effect: '✨', side_set: '🖼️', miu_set: '🐰', bubble: '💬', flower: '🌸', line_pack: '📜', model: '🎭' };
  // 'model': model đang dùng nằm ở Settings.pet_model, Rust đánh dấu sẵn `selected`.
  const SELECTABLE = ['skin', 'widget_deco', 'side_set', 'miu_set', 'bubble', 'flower', 'model'];

  const T = {
    vi: {
      title: 'Bộ sưu tập', subtitle: 'Thu thập và mở khoá những điều dễ thương cùng {pet}! 💗',
      candyLabel: 'Kẹo hiện tại', candy: 'kẹo', todayLabel: 'Hôm nay đã nhận', today: '{n} / {cap}',
      bubble: '{Pet} thích những chủ đề xinh xắn lắm đó! Cùng thu thập thêm nhé! 💗',
      howTitle: 'Cách nhận kẹo', earn: ['✅ Hoàn thành việc', '🍅 Xong phiên Tập trung', '🎮 Thắng mini game', '💗 Chat & vuốt đầu', '🏆 Đạt cột mốc'],
      noMoney: 'Không thể mua kẹo bằng tiền thật.',
      howTo: 'Kẹo đến từ việc đã xong, phiên Tập trung, chat, vuốt đầu và thắng game (có trần mỗi ngày). Mỗi cột mốc thân thiết +200 🍬. Không bán, không mua bằng tiền thật.',
      tabs: { skin: 'Skin widget', widget_deco: 'Trang trí', effect: 'Hiệu ứng', side_set: 'Nhân vật', miu_set: 'Miu mini', bubble: 'Bong bóng', flower: 'Bông hoa', line_pack: 'Câu thoại', model: 'Model' },
      rarity: { common: 'Thường', rare: 'Hiếm', epic: 'Sử thi', legendary: 'Huyền thoại' },
      using: 'Đang dùng', use: 'Dùng', buy: 'Đổi {price} 🍬', short: '🍬 Thiếu {n}', on: 'Đang bật', off: 'Bật',
      preview: 'Xem thử', free: 'Có sẵn', empty: 'Chưa có món nào ở nhóm này.', close: 'Đóng',
      download: '⬇ Tải về{size}', downloadingPct: 'Đang tải {n}%', update: '⬆ Cập nhật v{v}', installed: 'Đã tải xong {item}! Bấm Dùng để đổi nhé.',
      bought: '{item} là của {you} rồi! 🍬', sideOff: 'Bunny Lounge đang tắt: bật trong Cài đặt → 🎨 Diện mạo để thấy nhân vật.',
      modes: { gacha: 'Candy Capsule', shop: 'Cửa hàng kẹo', history: 'Lịch sử' },
      g: {
        bubble: 'Hôm nay thử vận may nào? Cùng quay thật nhiều đồ xinh nhé! 💗',
        sign: 'Sweet Dreams', signSub: 'Skin, nhân vật, bộ mặt Miu, hiệu ứng và cả model Live2D, quay là có!',
        pull1: 'Quay 1 lần', pull10: 'Quay 10 lần', short: 'Thiếu {n} 🍬',
        giftPull1: 'Quay 1 lần', giftPullN: 'Quay {n} lần', giftFree: '🎁 Miễn phí', giftNote: '🎁 Quà tặng: còn {n} lượt quay miễn phí', giftSure: 'Chắc chắn có 1 model mới!',
        ratesTitle: 'Tỷ lệ nhận vật phẩm',
        rateDesc: { common: 'Vật phẩm phổ biến', rare: 'Vật phẩm hiếm', epic: 'Vật phẩm rất hiếm', legendary: 'Model Live2D, cực hiếm' },
        pityRare: 'Còn {n} lượt chắc chắn ra Hiếm+', pityEpic: 'Còn {n} lượt chắc chắn ra Sử thi+',
        legendItem: '{name} {pct}% · trùng hoàn {c} 🍬',
        dupNote: 'Trùng món đã có: hoàn {c} / {r} / {e} 🍬', dupNoteLegend: ', Huyền thoại 10% giá',
        featured: 'Vật phẩm nổi bật trong máy kẹo', seeAll: 'Xem cửa hàng →',
        resultTitle: 'Kết quả quay', newTag: 'MỚI!', dupTag: 'Trùng · +{n} 🍬', pityTag: 'Bảo hiểm',
        sumNew: 'Mới {n} món', sumRefund: 'Hoàn {c} 🍬', again: 'Quay tiếp ({c} 🍬)', ok: 'Nhận',
        locked: 'Máy kẹo mở khi Bộ sưu tập có đủ {n} món.',
        histTitle: 'Lịch sử quay', histTotal: 'Tổng {n} lượt', histEmpty: 'Chưa quay lượt nào. Thử vận may đi!',
        histNew: 'Món mới', histDup: 'Trùng',
        rvCommon: 'Mở nào~!', rvRare: '✦ HIẾM! ✦', rvEpic: '🌟 SỬ THI! 🌟', rvLegendary: '🌈 HUYỀN THOẠI! 🌈', rvSkip: 'Bấm để bỏ qua',
      },
      err: {
        not_found: 'Không thấy món này.', no_gift: 'Hết lượt quay quà rồi.', already_owned: 'Đã có rồi mà~', not_enough: 'Còn thiếu {n} 🍬 nữa.',
        wrong_category: 'Món này không dùng ở đây được.', not_owned: 'Chưa có món này.', locked: 'Model này chưa mở khoá.',
        downloading: 'Đang tải model, xong sẽ tự đổi.', busy: 'Đang tải rồi, đợi chút nhé.',
        offline: 'Không kết nối được máy chủ tải model. Kiểm tra mạng rồi thử lại nhé.',
        no_package: 'Model này chưa được đăng để tải. Thử lại sau nhé.',
        bad_signature: 'Danh sách model không có chữ ký hợp lệ nên đã dừng tải cho an toàn.',
        bad_manifest: 'Danh sách model bị hỏng. Thử lại sau nhé.',
        bad_checksum: 'File tải về không khớp nên đã bỏ. Thử lại nhé.',
        bad_package: 'Gói model thiếu file nên đã bỏ. Thử lại sau nhé.', other: 'Không làm được: {e}',
      },
    },
    ja: {
      title: 'コレクション', subtitle: '{pet}といっしょに、かわいいものを集めて解放しよう！💗',
      candyLabel: '今のキャンディ', candy: '個', todayLabel: '今日もらった数', today: '{n} / {cap}',
      bubble: 'かわいいテーマ、大好き！いっしょに集めようね！💗',
      howTitle: 'キャンディのもらい方', earn: ['✅ タスク完了', '🍅 集中セッション', '🎮 ミニゲーム勝利', '💗 チャット・なでなで', '🏆 節目の達成'],
      noMoney: 'キャンディはお金では買えません。',
      howTo: 'キャンディはタスク完了・集中セッション・チャット・なでなで・ゲーム勝利でもらえます（1日の上限あり）。なかよし度の節目ごとに +200 🍬。販売も課金もありません。',
      tabs: { skin: 'ウィジェットスキン', widget_deco: 'デコ', effect: 'エフェクト', side_set: 'キャラクター', miu_set: 'ミニミウ', bubble: '吹き出し', flower: 'お花', line_pack: 'セリフ', model: 'モデル' },
      rarity: { common: 'ノーマル', rare: 'レア', epic: 'エピック', legendary: 'レジェンド' },
      using: '使用中', use: '使う', buy: '{price} 🍬 で交換', short: 'あと {n} 🍬', on: 'オン', off: 'オンにする',
      preview: 'プレビュー', free: '最初から', empty: 'このグループにはまだアイテムがありません。', close: '閉じる',
      download: '⬇ ダウンロード{size}', downloadingPct: 'ダウンロード中 {n}%', update: '⬆ v{v} に更新', installed: '{item} のダウンロード完了！「使う」で切り替えてね。',
      bought: '{item} は{you}のもの！🍬', sideOff: 'バニーラウンジはオフです：設定 → 🎨 Diện mạo でオンにするとキャラクターが見えます。',
      modes: { gacha: 'キャンディカプセル', shop: 'キャンディショップ', history: '履歴' },
      g: {
        bubble: '今日の運だめし、してみる？かわいいもの、いっぱい出るといいね！💗',
        sign: 'Sweet Dreams', signSub: 'スキン・キャラ・ミウの表情・エフェクト、Live2Dモデルも出るよ！',
        pull1: '1回まわす', pull10: '10回まわす', short: 'あと {n} 🍬',
        giftPull1: '1回まわす', giftPullN: '{n}回まわす', giftFree: '🎁 無料', giftNote: '🎁 プレゼント：無料ガチャ残り {n} 回', giftSure: '新しいモデルが必ず1つ！',
        ratesTitle: '排出率',
        rateDesc: { common: 'よく出るアイテム', rare: 'めずらしいアイテム', epic: 'とてもめずらしいアイテム', legendary: 'Live2Dモデル・超レア' },
        pityRare: 'あと {n} 回でレア以上確定', pityEpic: 'あと {n} 回でエピック以上確定',
        legendItem: '{name} {pct}% · ダブりは {c} 🍬 返却',
        dupNote: 'ダブりは {c} / {r} / {e} 🍬 返ってくるよ', dupNoteLegend: '（レジェンドは価格の10%）',
        featured: 'カプセルの注目アイテム', seeAll: 'ショップを見る →',
        resultTitle: '結果', newTag: 'NEW!', dupTag: 'ダブり · +{n} 🍬', pityTag: '天井',
        sumNew: '新しいアイテム {n} 個', sumRefund: '{c} 🍬 返却', again: 'もう一度 ({c} 🍬)', ok: 'OK',
        locked: 'コレクションのアイテムが {n} 個になるとカプセルが使えます。',
        histTitle: 'ガチャ履歴', histTotal: '合計 {n} 回', histEmpty: 'まだ一度もまわしていません。運だめししてみよう！',
        histNew: '新規', histDup: 'ダブり',
        rvCommon: 'オープン～！', rvRare: '✦ レア！ ✦', rvEpic: '🌟 エピック！ 🌟', rvLegendary: '🌈 レジェンド！ 🌈', rvSkip: 'タップでスキップ',
      },
      err: {
        not_found: 'アイテムが見つかりません。', no_gift: 'プレゼントのガチャは使い切ったよ。', already_owned: 'もう持ってるよ～', not_enough: 'あと {n} 🍬 足りないよ。',
        wrong_category: 'ここでは使えないアイテムです。', not_owned: 'まだ持っていません。', locked: 'このモデルはまだロックされています。',
        downloading: 'モデルをダウンロード中。終わったら自動で切り替わるよ。', busy: 'ダウンロード中だよ、ちょっと待ってね。',
        offline: 'モデルのダウンロードサーバーにつながりません。ネットワークを確認してもう一度試してね。',
        no_package: 'このモデルはまだ公開されていません。あとで試してね。',
        bad_signature: 'モデル一覧の署名が正しくないため、安全のためダウンロードを止めました。',
        bad_manifest: 'モデル一覧が壊れています。あとで試してね。',
        bad_checksum: 'ダウンロードしたファイルが一致しないため破棄しました。もう一度試してね。',
        bad_package: 'モデルのファイルが足りないため破棄しました。あとで試してね。', other: 'できませんでした: {e}',
      },
    },
    en: {
      title: 'Collection', subtitle: 'Collect and unlock cute things with {pet}! 💗',
      candyLabel: 'Candy now', candy: 'candy', todayLabel: 'Earned today', today: '{n} / {cap}',
      bubble: "I love pretty themes! Let's collect more together! 💗",
      howTitle: 'How to earn candy', earn: ['✅ Finish tasks', '🍅 Focus sessions', '🎮 Win mini games', '💗 Chat & headpats', '🏆 Reach milestones'],
      noMoney: 'Candy can never be bought with real money.',
      howTo: 'Candy comes from finished tasks, focus sessions, chat, headpats and game wins (daily caps apply). Every friendship milestone adds +200 🍬. Never sold, never bought with real money.',
      tabs: { skin: 'Widget skins', widget_deco: 'Decor', effect: 'Effects', side_set: 'Characters', miu_set: 'Mini Miu', bubble: 'Bubbles', flower: 'Flower', line_pack: 'Lines', model: 'Models' },
      rarity: { common: 'Common', rare: 'Rare', epic: 'Epic', legendary: 'Legendary' },
      using: 'In use', use: 'Use', buy: 'Trade {price} 🍬', short: '{n} 🍬 short', on: 'On', off: 'Turn on',
      preview: 'Preview', free: 'Included', empty: 'Nothing in this group yet.', close: 'Close',
      download: '⬇ Download{size}', downloadingPct: 'Downloading {n}%', update: '⬆ Update to v{v}', installed: '{item} is downloaded! Press Use to switch.',
      bought: '{item} is yours now! 🍬', sideOff: 'Bunny Lounge is off: turn it on in Settings → 🎨 Diện mạo to see the characters.',
      modes: { gacha: 'Candy Capsule', shop: 'Candy shop', history: 'History' },
      g: {
        bubble: "Feeling lucky today? Let's spin for lots of cute things! 💗",
        sign: 'Sweet Dreams', signSub: 'Skins, characters, Miu faces, effects and even Live2D models inside!',
        pull1: 'Spin once', pull10: 'Spin 10 times', short: '{n} 🍬 short',
        giftPull1: 'Spin once', giftPullN: 'Spin {n} times', giftFree: '🎁 Free', giftNote: '🎁 Gift: {n} free spins left', giftSure: 'One new model guaranteed!',
        ratesTitle: 'Drop rates',
        rateDesc: { common: 'Common items', rare: 'Rare items', epic: 'Very rare items', legendary: 'Live2D models, ultra rare' },
        pityRare: '{n} more spins to a sure Rare+', pityEpic: '{n} more spins to a sure Epic+',
        legendItem: '{name} {pct}% · duplicate refunds {c} 🍬',
        dupNote: 'Duplicates refund {c} / {r} / {e} 🍬', dupNoteLegend: ', Legendary 10% of price',
        featured: 'Featured in the capsule machine', seeAll: 'Open the shop →',
        resultTitle: 'Your capsules', newTag: 'NEW!', dupTag: 'Duplicate · +{n} 🍬', pityTag: 'Pity',
        sumNew: '{n} new', sumRefund: '{c} 🍬 refunded', again: 'Spin again ({c} 🍬)', ok: 'Collect',
        locked: 'The capsule machine opens once the Collection has {n} items.',
        histTitle: 'Spin history', histTotal: '{n} spins in total', histEmpty: 'No spins yet. Try your luck!',
        histNew: 'New', histDup: 'Duplicate',
        rvCommon: 'Open up~!', rvRare: '✦ RARE! ✦', rvEpic: '🌟 EPIC! 🌟', rvLegendary: '🌈 LEGENDARY! 🌈', rvSkip: 'Click to skip',
      },
      err: {
        not_found: 'No such item.', no_gift: 'No gift spins left.', already_owned: 'You already have it~', not_enough: 'Still {n} 🍬 short.',
        wrong_category: 'This item cannot be used here.', not_owned: "You don't have this item yet.", locked: 'This model is still locked.',
        downloading: 'Downloading the model; it switches over when done.', busy: 'Already downloading, hang on.',
        offline: "Can't reach the model download server. Check your connection and try again.",
        no_package: "This model hasn't been published for download yet. Try again later.",
        bad_signature: 'The model list has no valid signature, so the download was stopped to keep you safe.',
        bad_manifest: 'The model list is broken. Try again later.',
        bad_checksum: "The downloaded file didn't match, so it was discarded. Try again.",
        bad_package: 'The model package is missing files, so it was discarded. Try again later.', other: 'Could not do that: {e}',
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
    // Model tải riêng: đã mua mà chưa có trên máy thì tải trước, đang tải thì hiện %.
    if (item.cat === 'model' && item.progress != null) {
      return { kind: 'busy', label: fill(S.downloadingPct, { n: item.progress }), disabled: true };
    }
    if (item.cat === 'model' && !item.installed) {
      const size = sizeLabel(item.size, lang);
      return { kind: 'download', label: fill(S.download, { size: size ? ' (' + size + ')' : '' }), disabled: false };
    }
    if (item.cat === 'effect') {
      return item.on ? { kind: 'on', label: S.on, disabled: false } : { kind: 'off', label: S.off, disabled: false };
    }
    if (SELECTABLE.includes(item.cat)) {
      return item.selected ? { kind: 'using', label: S.using, disabled: true } : { kind: 'use', label: S.use, disabled: false };
    }
    return { kind: 'using', label: S.free, disabled: true };
  }

  /** Dung lượng tải: "2,8 MB" (vi) / "2.8 MB". Không biết thì rỗng. */
  function sizeLabel(bytes, lang) {
    const n = Number(bytes);
    if (!n) return '';
    const mb = (n / 1048576).toFixed(1);
    return (lang === 'vi' ? mb.replace('.', ',') : mb) + ' MB';
  }

  function rarityClass(r) {
    return ['common', 'rare', 'epic', 'legendary'].includes(r) ? 'r-' + r : 'r-common';
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
    // Model: ảnh preview.png khi có (Rust gửi đường dẫn); chưa có thì trang hiện 🎭.
    if (item.cat === 'model') return item.thumb || '';
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
  const RARITY_RANK = { common: 0, rare: 1, epic: 2, legendary: 3 };

  /** Nút quay: đủ kẹo thì bấm được, thiếu thì báo thiếu bao nhiêu; máy khoá thì tắt. */
  function pullState(cost, candy, enabled, lang) {
    const g = strings(lang).g;
    if (!enabled) return { ok: false, label: '🔒' };
    const need = (Number(cost) || 0) - (Number(candy) || 0);
    return need > 0 ? { ok: false, label: fill(g.short, { n: need }) } : { ok: true, label: '' };
  }

  /**
   * 🎁 Nút quay khi còn lượt quà (gacha.gift > 0): hai nút quay đổi sang quay quà
   * (1 lượt và tối đa 10 lượt), không cần kẹo. Hết quà thì { active: false }.
   */
  function giftState(gacha, lang) {
    const left = Math.max(0, Number(gacha && gacha.gift) || 0);
    if (!left || !(gacha && gacha.enabled)) return { active: false };
    const g = strings(lang).g;
    const many = Math.min(10, left);
    return {
      active: true,
      counts: [1, many],
      labels: [g.giftPull1, many === 1 ? g.giftPull1 : fill(g.giftPullN, { n: many })],
      cost: g.giftFree,
      note: fill(g.giftNote, { n: left }) + (gacha.giftPromise ? ' · ' + g.giftSure : ''),
    };
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
    const hasPic = (i) => (i.cat === 'side_set' || i.cat === 'miu_set' || (i.cat === 'model' && i.thumb) ? 0 : 1);
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

  const api = { sizeLabel, RARITY_RANK, MODES, pullState, giftState, summarize, featured, formatTime, CATS, TAB_ICONS, SELECTABLE, T, strings, fill, groupByCategory, sortItems, buttonState, rarityClass, errorText, swatch, swatchInk, swatchAccent, flowerGlyph, thumb, effectIcon };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.CollectionCore = api;
})(typeof window !== 'undefined' ? window : globalThis);
