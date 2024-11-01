# pickls

**(pronounced /ˈpɪkᵊlz/)**

<img src="https://github.com/user-attachments/assets/64765055-e9a8-45a6-b89a-eb91c2e32ac7" style="width: 200px">

## The General Purpose Language Server for Command-Line Linters and Formatters

Inspired by tools like [ale](https://github.com/dense-analysis/ale),
[null-ls](https://github.com/jose-elias-alvarez/null-ls.nvim),
[none-ls](https://github.com/nvimtools/none-ls.nvim), and
[diagnostic-languageserver](https://github.com/iamcco/diagnostic-languageserver),
and [conform](https://github.com/oberblastmeister/conform), `pickls` offers a
unified way to configure command-line linters and formatters with editor LSP
integration.

### Key Features

- Integrate command-line linting and formatting tools with your IDE.
- Configure multiple linters and formatters for any language.
- Ideal for projects with toolchains lacking native LSP integration.

## Why Use pickls?

- Avoid installing and configuring separate plugins or language servers for each
  tool in your workflow.
- Utilize a seamless LSP integration for command-line oriented toolchains.

## Installation

### Install pickls Binary

Ensure you have a recent `stable` Rust toolchain and the cargo binary directory
in your path:

```sh
cargo install pickls
```

### Running from Source

Consider using `pickls-debug-runner` to run from source, which is helpful for
development purposes.

## Configuration

Configuration lives in a couple of places. First, create a file named
`pickls.yaml` and place it in your `"$XDG_CONFIG_HOME"/pickls` directory.

Second, configure `pickls` within your editor through the LSP initialization
settings. Configuration details are available
[here](https://docs.rs/crate/pickls/latest/source/src/config.rs).

### Example pickls.yaml

```yaml
---
.linters:
  clang-format: &clang-format
languages:
  c: &c-settings
    formatters:
      - program: clang-format
        args: ["-"]
  cpp: *c-settings
  dockerfile:
    linters:
      - program: hadolint
        args:
          - --no-color
          - --format
          - tty
          - '-'
        description_match: 3
        line_match: 1
        pattern: '-:(\d+) [^ ]+ (\w+): (.*)'
        severity_match: 2
        use_stderr: false
        use_stdin: true
  markdown:
    formatters:
      - program: mdformat
        args:
          - --wrap
          - '80'
          - '-'
  python:
    root_markers:
      - .git
      - pyproject.toml
      - setup.py
      - mypy.ini
    formatters:
      - program: autoimport
        args: ["-"]
      - program: isort
        args: ["-", "-d"]
      - program: ruff
        args: ["check", "--exit-zero", "--fix", "--stdin-filename", "$filename"]
      - program: ruff
        args:
          - format
          - --stdin-filename
          - $filename
    linters:
      - program: mypy
        args:
          - --show-column-numbers
          - --show-error-end
          - --hide-error-codes
          - --hide-error-context
          - --no-color-output
          - --no-error-summary
          - --no-pretty
          - --shadow-file
          - $filename
          - /dev/stdin
          - $filename
        pattern: '(.*):(\d+):(\d+):\d+:(\d+): error: (.*)'
        filename_match: 1
        line_match: 2
        start_col_match: 3
        end_col_match: 4
        description_match: 5
        use_stderr: false
        use_stdin: true
      - program: ruff
        args:
          - check
          - --stdin-filename
          - $filename
        pattern: '(.*):(\d+):(\d+): (.*)'
        filename_match: 1
        line_match: 2
        start_col_match: 3
        description_match: 4
        use_stderr: false
        use_stdin: true
  sh: &sh
    linters:
      - program: shellcheck
        args: ["-f", "gcc", "-"]
        pattern: '(.*):(\d+):(\d+): (\w+): (.*)'
        filename_match: 1
        line_match: 2
        start_col_match: 3
        severity_match: 4
        description_match: 5
        use_stderr: false
        use_stdin: true
  shell script: *sh
  toml:
    linters:
      - program: tomllint
        args: ["-"]
        pattern: '(.*):(\d+):(\d+): error: (.*)'
        filename_match: 1
        line_match: 2
        start_col_match: 3
        description_match: 4
        use_stderr: true
        use_stdin: true
  yaml:
    linters:
      - program: yamllint
        args: ["-f", "parsable", "-"]
        pattern: '.*:(\d+):(\d+): \[(.*)\] (.*) \((.*)\)'
        line_match: 1
        start_col_match: 2
        severity_match: 3
        description_match: 4
        use_stderr: false
        use_stdin: true
```

Note the usage of YAML anchors and references in order to handle different
language names for the same formats.

### Zed

To use `pickls` in Zed, install the
[pickls-zed](https://github.com/wbbradley/pickls-zed) extension. Use the
following command:

```bash
git clone https://github.com/wbbradley/pickls-zed "$HOME"/src/pickls-zed
```

Note that Zed supports formatting via command-line out of the box (see
`format_on_save`), so you don't really need to use `pickls` for formatting in
Zed. However, I've included it in the configuration here for demonstration
purposes.

#### Example Zed Settings

```jsonc
{
  "format_on_save": "language_server",
  "languages": {
    "Python": {
      // Note that this implicitly disables Zed's built-in usage of Pyright.
      "language_servers": ["pickls"],
    }
  },
  "lsp": {
    "pickls": {
      "binary": {"path": "pickls", "arguments": ["zed"]},
    }
  }
}
```

### Neovim

Enable `pickls` for all Neovim buffers:

```lua
vim.api.nvim_create_autocmd({ "BufRead" }, {
  group = vim.api.nvim_create_augroup("pickls-bufread", { clear = true }),
  callback = function(_)
    if vim.fn.executable("pickls") ~= 0 then
      vim.lsp.start({
        name = "pickls",
        cmd = { "pickls", vim.api.nvim_buf_get_name(0) },
        root_dir = vim.fs.root(0, { ".git", "pyproject.toml", "setup.py", "Cargo.toml", "go.mod" }),
      }, {
        bufnr = 0,
        reuse_client = function(_, _) return false end,
      })
    else
      vim.notify("Pickls executable not found. See pickls-debug-runner for setup instructions.")
    end
  end,
})

vim.api.nvim_create_autocmd("BufWritePre", {
  callback = function() vim.lsp.buf.format({ bufnr = bufnr }) end
})
```

### VSCode

TODO: Provide VSCode setup instructions.
