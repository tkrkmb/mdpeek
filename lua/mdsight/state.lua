-- mdsightの実行時の状態。init.luaとrpc.luaで共有する。
return {
  bin = nil, -- setup({ bin = ... }) で設定された実行ファイルのパス
  token = nil, -- 起動トークン
  chan = nil, -- registerで記録したアプリのチャネルID
  proc = nil, -- { handle = vim.system の戻り値, exited = boolean }
  buf = nil, -- 対象バッファ
  win = nil, -- 対象ウィンドウ
  augroup = nil, -- 対象に張ったautocmdのグループID
  gen = 0, -- 対象世代
  version = 0, -- 版
  last_line = nil, -- 直前に送ったカーソル行
  away = false, -- 対象ウィンドウが、対象バッファ以外を表示しているか
  search_key = nil, -- 一致を求めたときの検索語、強調の有無、対象世代、版など
  search_matches = nil, -- 求めた一致 { line, byteidx, text } の配列
  search_sent = nil, -- 前回 mdsight_search を送ったときの search_key と現在の一致
  content_timer = nil, -- 本文送信のデバウンス用タイマー
  cursor_timer = nil, -- カーソル送信のスロットル用タイマー
  pending_line = nil, -- スロットル中に届いた、いちばん新しい行
}
