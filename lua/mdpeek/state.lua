-- mdpeekの実行時の状態。init.luaとrpc.luaで共有する。
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
}
