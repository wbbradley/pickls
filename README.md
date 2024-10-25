# pickls

<img src="https://github.com/user-attachments/assets/64765055-e9a8-45a6-b89a-eb91c2e32ac7" width="20em">

## Installing in Neovim

```lua
vim.api.nvim_create_autocmd({ "BufRead" }, {
  group = vim.api.nvim_create_augroup("pickls-bufread", { clear = true }),
  callback = function(_)
    if vim.fn.executable("pickls") ~= 0 then
      -- We found an executable for pickls.
      vim.lsp.set_log_level(vim.log.levels.INFO)
      vim.lsp.start({
        name = "pickls",
        cmd = { "pickls", vim.api.nvim_buf_get_name(0) },
        root_dir = vim.fs.root(0, { ".git", "pyproject.toml", "setup.py", "Cargo.toml", "go.mod" }),
        settings = {
          languages = {
            toml = {
              linters = {
                {
                  program = "tomllint",
                  args = { "-" },
                  pattern = "(.*):(\\d+):(\\d+): error: (.*)",
                  filename_match = 1,
                  line_match = 2,
                  start_col_match = 3,
                  description_match = 4,
                  use_stdin = true,
                  use_stderr = true,
                },
              },
            },
            sh = {
              linters = {
                {
                  program = "shellcheck",
                  args = {
                    "-f",
                    "gcc",
                    "-",
                  },
                  pattern = "(.*):(\\d+):(\\d+): (\\w+): (.*)",
                  filename_match = 1,
                  line_match = 2,
                  start_col_match = 3,
                  severity_match = 4,
                  description_match = 5,
                  use_stdin = true,
                  use_stderr = false,
                },
              },
            },
            python = {
              linters = {
                {
                  program = "mypy",
                  args = {
                    "--show-column-numbers",
                    "--show-error-end",
                    "--hide-error-codes",
                    "--hide-error-context",
                    "--no-color-output",
                    "--no-error-summary",
                    "--no-pretty",
                    "--shadow-file",
                    "$filename",
                    "/dev/stdin",
                    "$filename",
                  },
                  pattern = "(.*):(\\d+):(\\d+):\\d+:(\\d+): error: (.*)",
                  filename_match = 1,
                  line_match = 2,
                  start_col_match = 3,
                  end_col_match = 4,
                  description_match = 5,
                  use_stdin = true,
                  use_stderr = false,
                },
                {
                  program = "ruff",
                  args = {
                    "check",
                    "--stdin-filename",
                    "$filename",
                  },
                  pattern = "(.*):(\\d+):(\\d+): (.*)",
                  filename_match = 1,
                  line_match = 2,
                  start_col_match = 3,
                  description_match = 4,
                  use_stdin = true,
                  use_stderr = false,
                },
              },
            },
          },
        },
      }, {
        bufnr = 0,
        reuse_client = function(_, _)
          return false
        end,
      })
    else
      vim.notify(
        "unable to find 'pickls' executable. not registering pickls as a language server. " ..
        "see pickls-debug-runner for further instructions")
    end
  end,
})
```
