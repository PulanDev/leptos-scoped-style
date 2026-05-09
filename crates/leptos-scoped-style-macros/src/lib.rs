//! Compile Sass at macro expansion time.
//!
//! [`scss_file`] resolves its argument relative to the **invoking crate’s**
//! [`CARGO_MANIFEST_DIR`], so library crates (and apps) can use `@use` paths
//! the same way as `grass` on disk.

use std::cell::RefCell;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use grass_compiler::StdFs;
use proc_macro::TokenStream;
use quote::format_ident;
use syn::{parse_macro_input, LitStr};

use quote::__private::TokenStream as TokenStream2;

#[derive(Debug)]
struct FileTracker<'a> {
    files: RefCell<HashSet<PathBuf>>,
    fs: &'a dyn grass_compiler::Fs,
}

impl<'a> grass_compiler::Fs for FileTracker<'a> {
    fn is_dir(&self, path: &std::path::Path) -> bool {
        self.fs.is_dir(path)
    }

    fn is_file(&self, path: &std::path::Path) -> bool {
        self.fs.is_file(path)
    }

    fn read(&self, path: &std::path::Path) -> std::io::Result<Vec<u8>> {
        if let Ok(p) = std::fs::canonicalize(path) {
            self.files.borrow_mut().insert(p);
        }

        self.fs.read(path)
    }
}

fn track_files_for_rerun(files: &HashSet<PathBuf>) -> TokenStream2 {
    let mut out = quote::quote!();

    for (idx, file) in files.iter().enumerate() {
        let ident = format_ident!("__LEPTOS_SCOPED_STYLE_SCSS_DEP_{idx}");
        let file_name = file.to_string_lossy();
        out.extend::<TokenStream2>(quote::quote!(
            #[allow(dead_code)]
            const #ident: &str = include_str!(#file_name);
        ));
    }

    out
}

fn finish(css: String, files: &HashSet<PathBuf>) -> TokenStream {
    let deps = track_files_for_rerun(files);
    quote::quote!({
        #deps
        #css
    })
    .into()
}

/// Compile a Sass/SCSS file to CSS and expand to a `&'static str` expression.
///
/// `path` is joined with `CARGO_MANIFEST_DIR` from the crate that invokes this
/// macro (the one currently being compiled), **not** the leptos-scoped-style crate.
/// Use forward slashes in the literal (works on Windows too once joined).
///
/// Typical usage with [`leptos_scoped_style::ComponentStyle::css`][css]:
///
/// [css]: https://docs.rs/leptos-scoped-style/latest/leptos_scoped_style/struct.ComponentStyle.html#method.css
///
/// ```ignore
/// use leptos_scoped_style::{ComponentStyle, scss_file};
///
/// static STYLES: ComponentStyle = ComponentStyle::css(
///     "my-widget",
///     scss_file!("src/components/widget/widget.scss"),
/// );
/// ```
#[proc_macro]
pub fn scss_file(input: TokenStream) -> TokenStream {
    let lit = parse_macro_input!(input as LitStr);
    let rel = lit.value();

    let manifest_dir = match std::env::var("CARGO_MANIFEST_DIR") {
        Ok(v) => v,
        Err(_) => {
            let err = syn::Error::new(
                lit.span(),
                "scss_file!: CARGO_MANIFEST_DIR is not set (invoke from a Cargo-built crate)",
            );
            return syn::Error::into_compile_error(err).into();
        }
    };

    let abs = Path::new(&manifest_dir).join(rel.trim_start_matches('/'));

    let fs = FileTracker {
        files: RefCell::new(HashSet::new()),
        fs: &StdFs,
    };

    let options = grass_compiler::Options::default();
    let css = match grass_compiler::from_path(
        &abs,
        &options
            .fs(&fs)
            .style(grass_compiler::OutputStyle::Compressed),
    ) {
        Ok(css) => css,
        Err(e) => {
            let msg = format!(
                "Failed to compile Sass ({})\n{e}",
                abs.display()
            );
            let err = syn::Error::new(lit.span(), msg);
            return syn::Error::into_compile_error(err).into();
        }
    };

    let files = fs.files.into_inner();
    finish(css, &files)
}
