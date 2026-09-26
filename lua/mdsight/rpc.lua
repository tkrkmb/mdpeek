local state = require("mdsight.state")
local mdsight = require("mdsight")

-- アプリから呼べるのは、このモジュールが公開する関数だけにする。
local M = {}

local function cursor_line()
  local position = mdsight.target_cursor()
  return position and position[1] or 1
end

-- 世代と版が現在の値と一致し、対象ウィンドウが有効で対象バッファを表示しているか
local function is_current_target(gen, version)
  if type(gen) ~= "number" or type(version) ~= "number" then
    return false
  end
  if gen ~= state.gen or version ~= state.version then
    return false
  end
  return mdsight.target_shown() and vim.api.nvim_buf_is_valid(state.buf)
end

-- トークンを検証してチャネルを記録し、初期状態を返す。
function M.register(token, chan)
  if type(token) ~= "string" or type(state.token) ~= "string" or token ~= state.token then
    return nil
  end
  if type(chan) ~= "number" then
    return nil
  end
  if not (state.buf and vim.api.nvim_buf_is_valid(state.buf)) then
    return nil
  end

  state.chan = chan
  -- 起動前から検索の強調が出ていれば、登録の後に送る
  vim.schedule(function()
    mdsight.update_search()
  end)
  return {
    gen = state.gen,
    version = state.version,
    path = vim.api.nvim_buf_get_name(state.buf),
    lines = vim.api.nvim_buf_get_lines(state.buf, 0, -1, false),
    line = cursor_line(),
  }
end

-- アプリからのジャンプ要求。すべての検証に通ったときだけカーソルを動かす。
function M.jump(gen, version, line)
  if type(line) ~= "number" or not is_current_target(gen, version) then
    return false
  end
  if line < 1 or line > vim.api.nvim_buf_line_count(state.buf) then
    return false
  end

  vim.api.nvim_win_set_cursor(state.win, { line, 0 })
  vim.api.nvim_win_call(state.win, function()
    -- ジャンプ先が折り畳まれていたら開く
    if vim.fn.foldclosed(line) ~= -1 then
      vim.cmd("normal! zv")
    end
  end)
  -- このジャンプで起きる CursorMoved は送らない
  state.last_line = line
  return true
end

-- 絶対パスで、読めるファイルで、拡張子が md／markdown であること
local function is_openable_markdown(path)
  if type(path) ~= "string" or not path:match("^/") then
    return false
  end
  local extension = path:match("%.([%w]+)$")
  if extension == nil then
    return false
  end
  extension = extension:lower()
  if extension ~= "md" and extension ~= "markdown" then
    return false
  end
  return vim.fn.filereadable(path) == 1
end

-- アプリからのリンクを開く要求。すべての検証に通ったときだけ、対象ウィンドウで
-- 指定されたファイルを開く。開いた後のバッファが対象にできるものであれば、それを
-- 新しい対象にする。成功したら {gen, version} を、失敗したら false を返す。
function M.open(gen, version, path)
  if not is_current_target(gen, version) or not is_openable_markdown(path) then
    return false
  end

  local win = state.win
  local previous_buf = state.buf

  -- 開く前に、いまの対象のautocmdを外す
  -- (バッファがwipeされてBufWipeoutが走っても :MdSightClose にならないように)
  mdsight.detach_autocmds()

  local opened = pcall(vim.api.nvim_win_call, win, function()
    vim.cmd.edit(vim.fn.fnameescape(path))
  end)
  if not opened then
    -- 開けなかった。元の対象のautocmdを戻す
    mdsight.set_autocmds(previous_buf)
    vim.notify("mdsight: cannot open " .. path, vim.log.levels.WARN)
    return false
  end

  local new_buf = vim.api.nvim_win_get_buf(win)
  local ok, reason = mdsight.check_buf(new_buf)
  if not ok then
    -- 開けたが、対象にできるバッファではなかった。元の対象のままにし、
    -- 対象ウィンドウに対象にできるバッファが戻ってくるのを待つ
    vim.notify("mdsight: cannot preview this buffer: " .. reason, vim.log.levels.WARN)
    if vim.api.nvim_buf_is_valid(previous_buf) then
      mdsight.set_autocmds(previous_buf)
    else
      state.buf = nil
      mdsight.set_autocmds(nil)
    end
    state.away = true
    return false
  end

  mdsight.set_target(new_buf, win)
  return { gen = state.gen, version = state.version }
end

return M
