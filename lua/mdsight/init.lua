local state = require("mdsight.state")

local M = {}

local function warn(msg)
  vim.notify("mdsight: " .. msg, vim.log.levels.WARN)
end

-- 対象にできるバッファかどうかを調べる。使えない場合は理由を返す。
local function check_buf(buf)
  if vim.bo[buf].buftype ~= "" then
    return false, "buftype is not empty"
  end
  if vim.api.nvim_buf_get_name(buf) == "" then
    return false, "buffer has no file name"
  end
  if vim.bo[buf].filetype ~= "markdown" then
    return false, "filetype is not markdown"
  end
  return true, nil
end

local function make_token()
  local ok, bytes = pcall(vim.uv.random, 16)
  if ok and type(bytes) == "string" then
    return (bytes:gsub(".", function(c)
      return string.format("%02x", string.byte(c))
    end))
  end
  return vim.fn.sha256(tostring(vim.uv.hrtime()) .. tostring(vim.fn.getpid()))
end

local function socket()
  local sock = vim.v.servername
  if sock == nil or sock == "" then
    sock = vim.fn.serverstart()
  end
  return sock
end

-- 対象バッファの全行を送る。版を1増やす。
local function send_content()
  if not (state.chan and state.buf and vim.api.nvim_buf_is_valid(state.buf)) then
    return
  end
  state.version = state.version + 1
  pcall(vim.rpcnotify, state.chan, "mdsight_content", {
    gen = state.gen,
    version = state.version,
    path = vim.api.nvim_buf_get_name(state.buf),
    lines = vim.api.nvim_buf_get_lines(state.buf, 0, -1, false),
  })
end

-- カーソル行を送る。直前に送った行と同じなら送らない。
local function send_cursor(line)
  if not (state.chan and line and line ~= state.last_line) then
    return
  end
  state.last_line = line
  pcall(vim.rpcnotify, state.chan, "mdsight_cursor", { gen = state.gen, line = line })
end

local function stop_cursor_timer()
  if state.cursor_timer then
    state.cursor_timer:stop()
    state.cursor_timer:close()
    state.cursor_timer = nil
  end
  state.pending_line = nil
end

-- スロットルの周期ごとに、溜まっているいちばん新しい行を送る
local function flush_cursor()
  local line = state.pending_line
  state.pending_line = nil
  if line == nil then
    stop_cursor_timer()
    return
  end
  send_cursor(line)
end

-- 対象ウィンドウの、いまのカーソル行を送る
local function send_cursor_now()
  if not (state.win and vim.api.nvim_win_is_valid(state.win)) then
    return
  end
  local ok, position = pcall(vim.api.nvim_win_get_cursor, state.win)
  if ok then
    send_cursor(position[1])
  end
end

-- 50ms間隔のスロットルでカーソル行を送る
local function schedule_cursor()
  if not (state.win and vim.api.nvim_win_is_valid(state.win)) then
    return
  end
  local ok, position = pcall(vim.api.nvim_win_get_cursor, state.win)
  if not ok then
    return
  end
  local line = position[1]
  if line == state.last_line then
    return
  end

  if state.cursor_timer then
    -- 周期の途中なので、いちばん新しい行だけを覚えておく
    state.pending_line = line
    return
  end
  send_cursor(line)
  state.cursor_timer = vim.uv.new_timer()
  state.cursor_timer:start(50, 50, vim.schedule_wrap(flush_cursor))
end

-- 200msのデバウンスのあとに送る
local function schedule_content()
  if not state.content_timer then
    state.content_timer = vim.uv.new_timer()
  end
  state.content_timer:stop()
  state.content_timer:start(200, 0, vim.schedule_wrap(send_content))
end

-- Lua側の状態、autocmd、タイマーを解放する。世代と版は増え続けるので戻さない。
local function release()
  stop_cursor_timer()
  if state.content_timer then
    state.content_timer:stop()
    state.content_timer:close()
    state.content_timer = nil
  end
  if state.augroup then
    pcall(vim.api.nvim_del_augroup_by_id, state.augroup)
    state.augroup = nil
  end
  state.token = nil
  state.chan = nil
  state.proc = nil
  state.buf = nil
  state.win = nil
  state.last_line = nil
end

local function set_autocmds(buf)
  if state.augroup then
    pcall(vim.api.nvim_del_augroup_by_id, state.augroup)
  end
  state.augroup = vim.api.nvim_create_augroup("mdsight", { clear = true })
  -- 本文が変わったら、デバウンスしてから送る
  vim.api.nvim_create_autocmd({ "TextChanged", "TextChangedI" }, {
    group = state.augroup,
    buffer = buf,
    callback = schedule_content,
  })
  -- 対象ウィンドウでカーソルが動いたら、スロットルして送る
  vim.api.nvim_create_autocmd({ "CursorMoved", "CursorMovedI" }, {
    group = state.augroup,
    callback = function()
      if state.win and vim.api.nvim_get_current_win() == state.win then
        schedule_cursor()
      end
    end,
  })
  -- 対象ウィンドウが閉じたら、カーソル位置の送信を止める
  vim.api.nvim_create_autocmd("WinClosed", {
    group = state.augroup,
    pattern = tostring(state.win),
    callback = function()
      state.win = nil
      stop_cursor_timer()
    end,
  })
  -- 対象バッファが消えたら、:MdSightClose と同じ処理を行う
  vim.api.nvim_create_autocmd({ "BufWipeout", "BufDelete" }, {
    group = state.augroup,
    buffer = buf,
    callback = function()
      M.close()
    end,
  })
end

-- 対象を設定する。対象世代を1増やす。アプリが動いていれば、本文とカーソル行をすぐ送る。
local function set_target(buf, win)
  state.buf = buf
  state.win = win
  state.gen = state.gen + 1
  state.last_line = nil
  set_autocmds(buf)

  if state.proc then
    send_content()
    send_cursor_now()
  end
end

local function spawn(sock, token)
  local rec = { exited = false }
  local ok, handle = pcall(vim.system, { state.bin, "--nvim", sock, "--token", token }, {}, function()
    rec.exited = true
    vim.schedule(function()
      -- プロセスが終了したら、Lua側の状態を解放する
      if state.proc == rec then
        release()
      end
    end)
  end)
  if not ok then
    warn("failed to start " .. tostring(state.bin) .. ": " .. tostring(handle))
    return false
  end
  rec.handle = handle
  state.proc = rec
  return true
end

function M.setup(opts)
  opts = opts or {}
  state.bin = opts.bin

  vim.api.nvim_create_user_command("MdSight", function()
    M.open()
  end, { desc = "Preview the current markdown buffer with mdsight" })

  vim.api.nvim_create_user_command("MdSightClose", function()
    M.close()
  end, { desc = "Close the mdsight preview" })
end

function M.open()
  if type(state.bin) ~= "string" or state.bin == "" then
    warn('setup({ bin = "<path to the mdsight executable>" }) is required')
    return
  end

  local buf = vim.api.nvim_get_current_buf()
  local win = vim.api.nvim_get_current_win()
  local ok, reason = check_buf(buf)
  if not ok then
    warn("cannot preview this buffer: " .. reason)
    return
  end

  set_target(buf, win)

  if state.proc then
    -- すでにアプリが動いているので、起動はしない（本文とカーソル行はset_targetがすぐ送る）
    return
  end

  state.token = make_token()
  if not spawn(socket(), state.token) then
    release()
  end
end

function M.close()
  local rec = state.proc
  local chan = state.chan
  if chan then
    pcall(vim.rpcnotify, chan, "mdsight_close", {})
  end
  release()
  if rec and not rec.exited and rec.handle then
    -- 応答がなければプロセスを終了させる
    vim.defer_fn(function()
      if not rec.exited then
        pcall(function()
          rec.handle:kill("sigterm")
        end)
      end
    end, 1000)
  end
end

-- rpc.lua が、リンクを辿る要求(open)の実装に使う
M.check_buf = check_buf
M.set_autocmds = set_autocmds
M.set_target = set_target

return M
