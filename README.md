# lintls

## Installing in Neovim


```lua
vim.api.nvim_create_autocmd("BufNew", {
  group = vim.api.nvim_create_augroup("lintls-bufnew", { clear = true }),
  callback = function(event)
    vim.lsp.start({
      name = "lintls",
      cmd = { "lintls" },
      root_dir = vim.fs.root(0, { ".git", "pyproject.toml", "setup.py", "Cargo.toml", "go.mod" }),
    }, {
      bufnr = 0,
      reuse_client = function(_, _)
        return true
      end,
    })
  end,
})
```
