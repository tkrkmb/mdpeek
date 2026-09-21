local state = require("mdpeek.state")

-- アプリから呼べるのは、このモジュールが公開する関数だけにする。
local M = {}

local function cursor_line()
  if state.win and vim.api.nvim_win_is_valid(state.win) then
    local ok, pos = pcall(vim.api.nvim_win_get_cursor, state.win)
    if ok then
      return pos[1]
    end
  end
  return 1
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
  return {
    gen = state.gen,
    version = state.version,
    path = vim.api.nvim_buf_get_name(state.buf),
    lines = vim.api.nvim_buf_get_lines(state.buf, 0, -1, false),
    line = cursor_line(),
  }
end

return M
