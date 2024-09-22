# lintlsp

## Installing in Neovim


```lua
vim.api.nvim_create_autocmd("BufNew", {
  group = vim.api.nvim_create_augroup("lintlsp-bufnew", { clear = true }),
  callback = function(event)
    vim.lsp.start({
      name = "lintlsp",
      cmd = { "lintlsp" },
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
