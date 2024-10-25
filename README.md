# pickls

(pronounced /ˈpɪkᵊlz/)

<img src="https://github.com/user-attachments/assets/64765055-e9a8-45a6-b89a-eb91c2e32ac7" style="width: 200px">

## The General Purpose Language Server for Command-Line Linters and Formatters

In the spirit of [ale](https://github.com/dense-analysis/ale),
[null-ls](https://github.com/jose-elias-alvarez/null-ls.nvim),
[none-ls](https://github.com/nvimtools/none-ls.nvim), and
[diagnostic-languageserver](https://github.com/iamcco/diagnostic-languageserver), `pickls` unifies
configuration of classic command-line linters and formatters\[0\].

You should use `pickls` if you

- are working in a project where your languages/toolchain lacks LSP integration, but you
- have command-line linting and formatting tools you'd like to integrate with your IDE.
- have tried the other tools in this category and are seeking an alternative.

`pickls` allows for configuration of multiple linters and formatters for any language, and provides
a unified way to run these tools on a per-file basis. `pickls` is designed to fit seamlessly into
any IDE that supports LSP.

\[0\] Formatters are not yet supported, but work is underway.

## Why?

Because my editor already supports LSP, and I want to avoid having to find, install and configure
(or build myself) separate custom plugins for each linter and formatter.

## Installation

### Installing the pickls binary

The following command assumes you have a working Rust toolchain installed, and the cargo binary
directory is in your path.

```
cargo install pickls
```

#### Running from source

See `pickls-debug-runner` if you'd like to run from source.

## Configuration

Configuration happens inline in the LSP initialization settings of your IDE. Here is an example
structure encoded in Lua.

```lua
# This is a Lua representation of the configuration, your editor might prefer JSON, etc.
{
  languages = {
    python = {
      linters = {
        {
          -- I have had great success running mypy as its own language server, so this just runs it as a linter.
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
          -- Don't do this, just use `ruff server` https://docs.astral.sh/ruff/editors/.
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
    sh = {
      linters = {
        {
          -- https://www.shellcheck.net/
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
    toml = {
      linters = {
        {
          -- https://pypi.org/project/tomllint/
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
  },
}
```

### Configuring your editor

Once you have `pickls` installed, you can configure it to run within your editor.

#### Neovim

```lua
vim.api.nvim_create_autocmd({ "BufRead" }, {
  group = vim.api.nvim_create_augroup("pickls-bufread", { clear = true }),
  callback = function(_)
    if vim.fn.executable("pickls") ~= 0 then
      -- We found an executable for pickls.
      vim.lsp.start({
        name = "pickls",
        cmd = { "pickls", vim.api.nvim_buf_get_name(0) },
        -- Feel free to extend this list to include any other "root" indicators
        -- you'd like to use. The "root_dir" determines the working directory for
        -- the `pickls` process.
        root_dir = vim.fs.root(0, { ".git", "pyproject.toml", "setup.py", "Cargo.toml", "go.mod" }),
        settings = {
          -- ...See configuration in README above...
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

#### Zed

TODO: add instructions for zed

#### VSCode

TODO: add instructions for vscode
