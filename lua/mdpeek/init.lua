local state = require("mdpeek.state")

local M = {}

local function warn(msg)
  vim.notify("mdpeek: " .. msg, vim.log.levels.WARN)
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

-- Lua側の状態、autocmd、タイマーを解放する。世代と版は増え続けるので戻さない。
local function release()
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
  state.augroup = vim.api.nvim_create_augroup("mdpeek", { clear = true })
  -- 対象バッファが消えたら、:MdPeekClose と同じ処理を行う
  vim.api.nvim_create_autocmd({ "BufWipeout", "BufDelete" }, {
    group = state.augroup,
    buffer = buf,
    callback = function()
      M.close()
    end,
  })
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

  vim.api.nvim_create_user_command("MdPeek", function()
    M.open()
  end, { desc = "Preview the current markdown buffer with mdpeek" })

  vim.api.nvim_create_user_command("MdPeekClose", function()
    M.close()
  end, { desc = "Close the mdpeek preview" })
end

function M.open()
  if type(state.bin) ~= "string" or state.bin == "" then
    warn('setup({ bin = "<path to the mdpeek executable>" }) is required')
    return
  end

  local buf = vim.api.nvim_get_current_buf()
  local win = vim.api.nvim_get_current_win()
  local ok, reason = check_buf(buf)
  if not ok then
    warn("cannot preview this buffer: " .. reason)
    return
  end

  -- 対象を設定する。対象世代を1増やし、本文もこの時点の版として送る。
  state.buf = buf
  state.win = win
  state.gen = state.gen + 1
  state.version = state.version + 1
  state.last_line = nil
  set_autocmds(buf)

  if state.proc then
    -- すでにアプリが動いているので、起動しない
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
    pcall(vim.rpcnotify, chan, "mdpeek_close", {})
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

return M
