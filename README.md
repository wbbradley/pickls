# pickls

(pronounced /ˈpɪkᵊlz/)

<img src="https://github.com/user-attachments/assets/64765055-e9a8-45a6-b89a-eb91c2e32ac7" style="width: 200px">

## The General Purpose Language Server for Command-Line Linters and Formatters

In the spirit of [ale](https://github.com/dense-analysis/ale),
[null-ls](https://github.com/jose-elias-alvarez/null-ls.nvim),
[none-ls](https://github.com/nvimtools/none-ls.nvim), and
[diagnostic-languageserver](https://github.com/iamcco/diagnostic-languageserver),
`pickls` unifies configuration of classic command-line linters and formatters.

You should use `pickls` if you

- work in a project with a toolchain lacking LSP integration, and you
- have command-line linting and formatting tools you'd like to integrate with
  your IDE.
- Or, maybe you have tried the other tools in this category and are seeking an
  alternative.

`pickls` allows for configuration of multiple linters and formatters for any
language, and provides a unified way to run these tools on a per-file basis.
`pickls` is designed to fit seamlessly into any IDE that supports LSP.

## Why?

Because my editor already supports LSP, but I use a command-line oriented
toolchain, and I want to avoid having to find, install and configure (or build
myself) separate custom plugins or Language Servers for each tool in my
workflow.

## Installation

### Installing the pickls binary

The following command assumes you have a working recent `stable` Rust toolchain
installed, and the cargo binary directory is in your path.

```
cargo install pickls
```

#### Running from source

See `pickls-debug-runner` if you'd like to run from source. This is super useful
for developing on `pickls` itself.

## Configuration (aka Initialization Options)

Configuration happens inline in the LSP initialization settings of your IDE.
Documentation of the precise configuration syntax can be found
[here](https://docs.rs/crate/pickls/latest/source/src/config.rs).

Here is an example configuration encoded in JSON. Note that this is an example
for [Zed](https://zed.dev). Unfortunately, different editors use different
`language_id`s for the same language, so you may need to tweak this
configuration to match your editor's Language IDs.

````json
{
  "site": "zed",
  "languages": {
    "shell script": {
      "linters": [
        {
          "program": "shellcheck",
          "args": ["-f", "gcc", "-"],
          "pattern": "(.*):(\\d+):(\\d+): (\\w+): (.*)",
          "filename_match": 1,
          "line_match": 2,
          "start_col_match": 3,
          "severity_match": 4,
          "description_match": 5,
          "use_stdin": true,
          "use_stderr": false
        }
      ]
    },
    "dockerfile": {
      "linters": [
        {
          "program": "hadolint",
          "args": ["--no-color", "--format", "tty", "-"],
          "pattern": "-:(\\d+) [^ ]+ (\\w+): (.*)",
          "line_match": 1,
          "severity_match": 2,
          "description_match": 3,
          "use_stdin": true,
          "use_stderr": false
        }
      ]
    },
    "yaml": {
      "linters": [
        {
          "program": "yamllint",
          "args": ["-f", "parsable", "-"],
          "pattern": ".*:(\\d+):(\\d+): \\[(.*)\\] (.*) \\((.*)\\)",
          "filename_match": null,
          "line_match": 1,
          "start_col_match": 2,
          "severity_match": 3,
          "description_match": 4,
          "use_stdin": true,
          "use_stderr": false
        }
      ]
    },
    "toml": {
      "linters": [
        {
          "program": "tomllint",
          "args": ["-"],
          "pattern": "(.*):(\\d+):(\\d+): error: (.*)",
          "filename_match": 1,
          "line_match": 2,
          "start_col_match": 3,
          "description_match": 4,
          "use_stdin": true,
          "use_stderr": true
        }
      ]
    },
    "markdown": {
      "formatters": [
        {
          "program": "mdformat",
          "args": [ "--wrap", "80", "-" ]
        }
      ]
    },
    "python": {
      "root_markers": [".git", "pyproject.toml", "setup.py", "mypy.ini"],
      "formatters": [
        {
          "program": "autoimport",
          "args": ["-"]
        },
        {
          "program": "isort",
          "args": ["-", "-d"]
        },
        {
          "program": "ruff",
          "args": [
            "check",
            "--exit-zero",
            "--fix",
            "--unsafe-fixes",
            "--stdin-filename",
            " $filename"
          ]
        },
        {
          "program": "ruff",
          "args": ["format", "--stdin-filename", "$filename"]
        }
      ],
      "linters": [
        {
          "program": "mypy",
          "args": [
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
            "$filename"
          ],
          "pattern": "(.*):(\\d+):(\\d+):\\d+:(\\d+): error: (.*)",
          "filename_match": 1,
          "line_match": 2,
          "start_col_match": 3,
          "end_col_match": 4,
          "description_match": 5,
          "use_stdin": true,
          "use_stderr": false
        },
        {
          "program": "ruff",
          "args": ["check", "--stdin-filename", "$filename"],
          "pattern": "(.*):(\\d+):(\\d+): (.*)",
          "filename_match": 1,
          "line_match": 2,
          "start_col_match": 3,
          "description_match": 4,
          "use_stdin": true,
          "use_stderr": false
        }
      ]
    }
  }
}
```

```lua
-- This is a Lua representation of the configuration, your editor might prefer JSON, etc.
{
  -- `site` is purely for logging purposes, can be any string.
  site = "neovim",
  languages = {
    python = {
      linters = {
        {
          -- I have not had great success running mypy as its own language server, so this runs
          -- it as a traditional linter.
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
    dockerfile = {
      linters = {
        {
          -- See https://github.com/hadolint/hadolint
          program = "hadolint",
          args = {
            "--no-color",
            "--format",
            "tty",
            "-",
          },
          pattern = "-:(\\d+) [^ ]+ (\\w+): (.*)",
          line_match = 1,
          severity_match = 2,
          description_match = 3,
          use_stdin = true,
          use_stderr = false,
        },
      },
    },
    toml = {
      linters = {
        {
          -- See https://pypi.org/project/tomllint/
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
````

### Configuring your editor

Once you have `pickls` installed, you can configure it to run within your
editor.

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

Currently to get `pickls` running in Zed, you'll need to install the Zed
Extension `pickls-zed`. The only way to do that at the moment is to install it
as a Dev Extension from [here](https://github.com/wbbradley/pickls-zed).

```bash
git clone https://github.com/wbbradley/pickls-zed
```

Add the following example settings to your Zed settings (typically found in
`"$HOME"/.local/config/zed/settings.json`).

```jsonc
{
  "languages": {
    "TOML": {
      "language_servers": ["pickls"]
    },
    "Python": {
      "language_servers": ["pickls"],
    }
  }
  "lsp": {
    "pickls": {
      "binary": { "path": "pickls", "arguments": ["zed"] },
      "initialization_options": {
        "site": "zed",

        "languages": {
          "dockerfile": {
            "linters": [
              {
                "program": "hadolint",
                "args": ["--no-color", "--format", "tty", "-"],
                "pattern": "-:(\\d+) [^ ]+ (\\w+): (.*)",
                "line_match": 1,
                "severity_match": 2,
                "description_match": 3,
                "use_stdin": true,
                "use_stderr": false
              }
            ]
          },
          "toml": {
            "linters": [
              {
                "program": "tomllint",
                "args": ["-"],
                "pattern": "(.*):(\\d+):(\\d+): error: (.*)",
                "filename_match": 1,
                "line_match": 2,
                "start_col_match": 3,
                "description_match": 4,
                "use_stdin": true,
                "use_stderr": true
              }
            ]
          },
          "shell script": {
            "linters": [
              {
                "program": "shellcheck",
                "args": ["-f", "gcc", "-"],
                "pattern": "(.*):(\\d+):(\\d+): (\\w+): (.*)",
                "filename_match": 1,
                "line_match": 2,
                "start_col_match": 3,
                "severity_match": 4,
                "description_match": 5,
                "use_stdin": true,
                "use_stderr": false
              }
            ]
          },
          "python": {
            "root_markers": [".git", "pyproject.toml", "setup.py", "mypy.ini"],
            "linters": [
              {
                "program": "mypy",
                "args": [
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
                  "$filename"
                ],
                "pattern": "(.*):(\\d+):(\\d+):\\d+:(\\d+): error: (.*)",
                "filename_match": 1,
                "line_match": 2,
                "start_col_match": 3,
                "end_col_match": 4,
                "description_match": 5,
                "use_stdin": true,
                "use_stderr": false
              },
              {
                "program": "ruff",
                "args": ["check", "--stdin-filename", "$filename"],
                "pattern": "(.*):(\\d+):(\\d+): (.*)",
                "filename_match": 1,
                "line_match": 2,
                "start_col_match": 3,
                "description_match": 4,
                "use_stdin": true,
                "use_stderr": false
              }
            ]
          },
          "markdown": {
            "linters": [
              {
                "program": "pymarkdown",
                "args": ["scan-stdin"],
                "pattern": "(.*):(\\d+):(\\d+): (.*)",
                "filename_match": 1,
                "line_match": 2,
                "start_col_match": 3,
                "description_match": 4,
                "use_stdin": true,
                "use_stderr": false
              }
            ]
          }
        }
      }
    }
  }
}
```

#### VSCode

TODO: add instructions for vscode
