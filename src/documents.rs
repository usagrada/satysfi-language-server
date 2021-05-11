use std::{collections::HashMap, path::PathBuf};

use itertools::Itertools;
use lspower::lsp::Url;
use satysfi_parser::{Cst, CstText, LineCol, Rule, Span};

/// オンメモリで取り扱うデータをまとめたデータ構造。
#[derive(Debug, Default)]
pub struct DocumentCache(pub HashMap<Url, DocumentData>);

impl DocumentCache {
    /// dependencies の中のパッケージについてパースし、 Environment 情報の登録を行う。
    /// この操作は再帰的に行う。
    pub fn register_dependencies_recursive(&mut self, deps: &[Dependency]) {
        todo!()
    }

    /// dependencies の中のパッケージについてパースし、 Environment 情報の登録を行う。
    /// 既に同一の Url があればスキップする。
    pub fn register_dependency(&mut self, dep: &Dependency) {
        todo!()
    }
}

/// 一つのファイルに関するデータを纏めたデータ構造。
#[derive(Debug)]
pub enum DocumentData {
    /// パーサによって正常にパースできたデータ。
    Parsed {
        /// パース結果の具象構文木 + テキスト本体。
        csttext: CstText,
        /// 変数やコマンドに関する情報。
        environment: Environment,
    },

    /// パーサによってパースできなかったデータ。
    NotParsed {
        /// テキスト本体。
        text: String,
        /// エラー箇所。
        linecol: LineCol,
        /// エラー箇所にて期待するパターン（終端記号）列。
        expect: Vec<&'static str>,
    },
}

impl DocumentData {
    /// テキストから新たな DocumentData を作成する。
    pub fn new(text: &str, url: &Url) -> DocumentData {
        match CstText::parse(text, satysfi_parser::grammar::program) {
            Ok(csttext) => {
                let environment = Environment::new(&csttext, &url);
                DocumentData::Parsed {
                    csttext,
                    environment,
                }
            }
            Err((linecol, expect)) => {
                let text = text.to_owned();
                DocumentData::NotParsed {
                    text,
                    linecol,
                    expect,
                }
            }
        }
    }
}

/// 変数やコマンドに関する情報。
#[derive(Debug, Default)]
pub struct Environment {
    pub dependencies: Vec<Dependency>,
    pub modules: Vec<Module>,
    /// package にて定義された変数。
    pub variables: Vec<PackageComponent<Variable>>,
    /// package にて定義された型。
    pub types: Vec<PackageComponent<CustomType>>,
    /// package にて定義されたヴァリアント。
    pub variants: Vec<PackageComponent<Variant>>,
    /// package にて定義されたインラインコマンド。
    pub inline_cmds: Vec<PackageComponent<InlineCmd>>,
    /// package にて定義されたブロックコマンド。
    pub block_cmds: Vec<PackageComponent<BlockCmd>>,
    /// package にて定義された数式コマンド。
    pub math_cmds: Vec<PackageComponent<MathCmd>>,
}

impl Environment {
    fn new(csttext: &CstText, url: &Url) -> Environment {
        let dependencies = Dependency::extract(csttext, url);
        // let modules = Module::extract(csttext);
        let modules = vec![];
        // let types = CustomType::extract_from_package(csttext);
        let types = vec![];
        // let variants = Variant::extract_from_package(csttext);
        let variants = vec![];
        let variables = Variable::extract_from_package(csttext);
        let inline_cmds = InlineCmd::extract_from_package(csttext);
        let block_cmds = BlockCmd::extract_from_package(csttext);
        let math_cmds = vec![];
        // let math_cmds = MathCmd::extract_from_package(csttext);
        Environment {
            dependencies,
            modules,
            types,
            variants,
            variables,
            inline_cmds,
            block_cmds,
            math_cmds,
        }
    }
}

#[derive(Debug)]
pub struct Dependency {
    /// パッケージ名。
    name: String,
    /// require か import か。
    kind: DependencyKind,
    /// `@require:` や `@import` が呼ばれている場所。
    definition: Span,
    /// 実際のファイルパス。パスを解決できなかったら None を返す。
    url: Option<Url>,
}

#[derive(Debug)]
pub enum DependencyKind {
    Require,
    Import,
}

impl Dependency {
    /// 具象構文木からパッケージ情報を取り出す。
    fn extract(csttext: &CstText, url: &Url) -> Vec<Dependency> {
        let mut deps = vec![];
        let home_path = std::env::var("HOME").map(PathBuf::from).ok();
        let file_path = url.to_file_path().ok();

        let program = &csttext.cst;

        let require_packages = program
            .pickup(Rule::header_require)
            .into_iter()
            .map(|require| require.inner.get(0).unwrap());

        let import_packages = program
            .pickup(Rule::header_import)
            .into_iter()
            .map(|import| import.inner.get(0).unwrap());

        // require 系のパッケージの依存関係追加
        if let Some(home_path) = home_path {
            let dist_path = home_path.join(".satysfi/dist/packages");

            let require_dependencies = require_packages.map(|pkg| {
                let pkgname = csttext.get_text(pkg);
                // TODO: consider satyg file
                let pkg_path = dist_path.join(format!("{}.satyh", pkgname));
                let url = if pkg_path.exists() {
                    Url::from_file_path(pkg_path).ok()
                } else {
                    None
                };
                Dependency {
                    name: pkgname.to_owned(),
                    kind: DependencyKind::Require,
                    definition: pkg.span,
                    url,
                }
            });

            deps.extend(require_dependencies);
        }

        if let Some(file_path) = file_path {
            // TODO: add validate
            let parent_path = file_path.parent().unwrap();

            let import_dependencies = import_packages.map(|pkg| {
                let pkgname = csttext.get_text(pkg);
                // TODO: consider satyg file
                let pkg_path = parent_path.join(format!("{}.satyh", pkgname));
                let url = if pkg_path.exists() {
                    Url::from_file_path(pkg_path).ok()
                } else {
                    None
                };
                Dependency {
                    name: pkgname.to_owned(),
                    kind: DependencyKind::Import,
                    definition: pkg.span,
                    url,
                }
            });

            deps.extend(import_dependencies);
        }

        deps
    }
}

#[derive(Debug)]
pub struct Module {
    /// Module の名前。
    pub name: String,
    /// module にて定義された変数。
    pub variables: Vec<Variable>,
    /// module にて定義された型。
    pub types: Vec<ModuleComponent<CustomType>>,
    /// module にて定義されたヴァリアント。
    pub variants: Vec<ModuleComponent<Variant>>,
    /// module にて定義されたインラインコマンド。
    pub inline_cmds: Vec<ModuleComponent<InlineCmd>>,
    /// module にて定義されたブロックコマンド。
    pub block_cmds: Vec<ModuleComponent<BlockCmd>>,
    /// module にて定義された数式コマンド。
    pub math_cmds: Vec<ModuleComponent<MathCmd>>,
}

impl Module {
    fn extract(csttext: &CstText) -> Vec<Module> {
        todo!()
    }
}

/// package 内で定義された変数やコマンドなど。
#[derive(Debug)]
pub struct PackageComponent<T> {
    /// 可視性。外から見えるかどうか。
    pub visibility: PackageVisibility,
    /// 本体。
    pub body: T,
    /// スコープ。すなわち、対象となるファイルの中で、
    /// その変数や型、コマンドなどを使うことができる領域。
    pub scope: Span,
    /// 定義がどこにあるか (position)。
    pub pos_definition: usize,
}

/// module 内で定義された変数やコマンドなど。
#[derive(Debug)]
pub struct ModuleComponent<T> {
    /// 可視性。外から見えるかどうか。
    pub visibility: ModuleVisibility,
    /// 本体。
    pub body: T,
    /// スコープ。すなわち、対象となるファイルの中で、
    /// その変数や型、コマンドなどを使うことができる領域。
    pub scope: Span,
    /// sig 内部で declaration しているとき、その declaration がどこにあるか (position)。
    pub pos_declaration: Option<usize>,
    /// 定義がどこにあるか (position)。
    pub pos_definition: usize,
}

#[derive(Debug)]
pub struct Variable {
    /// 変数名。
    pub name: String,
    /// 変数の型（既知の場合）。
    pub type_: Option<String>,
    /// let 式に型情報を書いている場合、その場所。
    pub type_declaration: Option<Span>,
}

impl Variable {
    /// パッケージの CST + Text を与えて、パッケージ内にある変数定義を羅列する。
    fn extract_from_package(csttext: &CstText) -> Vec<PackageComponent<Variable>> {
        // TODO: let 式で直接定義される変数だけでなく、 argument も含めるようにする
        csttext
            .cst
            .pickup(Rule::let_stmt)
            .into_iter()
            .filter(|&cst| {
                if let Some(parent) = Variable::find_parent(csttext, cst) {
                    parent != Rule::module_stmt
                } else {
                    false
                }
            })
            .map(|cst| Variable::new_package_variable(csttext, cst))
            .concat()
    }

    /// パッケージの CST + Text 及び
    /// 対象となる let_stmt の CST を与えて、
    /// パッケージ内にある変数定義を羅列する。
    fn new_package_variable(csttext: &CstText, cst_stmt: &Cst) -> Vec<PackageComponent<Variable>> {
        let visibility = match Variable::find_parent(csttext, cst_stmt) {
            Some(Rule::bind_stmt) => PackageVisibility::Binded,
            Some(Rule::preamble) => PackageVisibility::Public,
            _ => unreachable!(),
        };
        let bodies = Variable::new(csttext, cst_stmt);
        let pos_definition = cst_stmt.inner[0].span.start;
        let scope = {
            let start = cst_stmt.span.end;
            let end = if visibility == PackageVisibility::Binded {
                // let 式で束縛された変数は、その let 式の bind が終了すれば無効となる
                if let Some(parent) = csttext.cst.get_parent(cst_stmt) {
                    // parent は let 式の bind がかかった expr
                    parent.span.end
                } else {
                    // 見つからなかったのでスコープを短めにする
                    cst_stmt.span.end
                }
            } else {
                // public な変数はそのファイルが終了するまで有効
                csttext.cst.span.end
            };
            Span { start, end }
        };
        bodies
            .into_iter()
            .map(|body| PackageComponent {
                visibility,
                body,
                pos_definition,
                scope,
            })
            .collect()
    }

    /// cst_stmt で定義される変数の列を返す。
    /// let_stmt では複数の変数が一気に登録される可能性があるため。
    fn new(csttext: &CstText, cst_stmt: &Cst) -> Vec<Variable> {
        let pattern = &cst_stmt.inner[0];
        let vars = pattern.pickup(Rule::var);
        vars.into_iter()
            .map(|cst| {
                let name = csttext.get_text(cst).to_owned();
                Variable {
                    name,
                    type_: None,
                    type_declaration: None,
                }
            })
            .collect()
    }

    /// その変数定義 (let_stmt) の親が
    /// - Rule::preamble
    /// - Rule::module_stmt
    /// - Rule::bind_stmt
    /// のいずれであるか判定する。
    fn find_parent(csttext: &CstText, cst: &Cst) -> Option<Rule> {
        let start_pos = cst.span.start;
        for parent in csttext.cst.dig(start_pos) {
            let rule = match parent.rule {
                // 式中の bind であることが確定
                Rule::bind_stmt => Rule::bind_stmt,
                // モジュール内定義であることが確定
                Rule::module_stmt => Rule::module_stmt,
                // preamble での定義であることが確定
                Rule::preamble => Rule::preamble,
                _ => continue,
            };
            return Some(rule);
        }
        None
    }
}

#[derive(Debug)]
pub struct CustomType {
    /// 型名。
    pub name: String,
    /// 型の定義。
    pub definition: String,
}

impl CustomType {
    /// パッケージの CST + Text を与えて、パッケージ内にある型定義を羅列する。
    fn extract_from_package(csttext: &CstText) -> Vec<PackageComponent<CustomType>> {
        let csts_type_stmt = csttext
            .cst
            .pickup(Rule::type_stmt)
            .into_iter()
            .filter(|&cst| {
                if let Some(parent) = CustomType::find_parent(cst) {
                    parent != Rule::module_stmt
                } else {
                    false
                }
            })
            .map(|cst| CustomType::new(csttext, cst));
        todo!()
    }

    /// その変数定義 (type_stmt) の親が
    /// - Rule::preamble
    /// - Rule::module_stmt
    /// のいずれであるか判定する。
    fn find_parent(cst: &Cst) -> Option<Rule> {
        let start_pos = cst.span.start;
        for parent in cst.dig(start_pos) {
            let rule = match parent.rule {
                // モジュール内定義であることが確定
                Rule::module_stmt => Rule::module_stmt,
                // preamble での定義であることが確定
                Rule::preamble => Rule::preamble,
                _ => continue,
            };
            return Some(rule);
        }
        None
    }

    /// パッケージの CstText + その中のモジュールの Cst を与えて、
    /// モジュール内にある型定義を羅列する。
    fn extract_from_module(csttext: &CstText, cst: &Cst) -> Vec<ModuleComponent<CustomType>> {
        todo!()
    }

    fn new(csttext: &CstText, cst: &Cst) -> CustomType {
        todo!()
    }
}

#[derive(Debug)]
pub struct Variant {
    /// variant 名。
    pub name: String,
    /// その Variant を持つ型の名前。
    pub type_name: String,
}

#[derive(Debug)]
pub struct InlineCmd {
    /// コマンド名。
    pub name: String,
    /// 型情報。
    pub type_: Option<Vec<String>>,
    /// 型情報の載っている場所。
    pub type_declaration: Option<Span>,
}

impl InlineCmd {
    /// パッケージの CST + Text を与えて、パッケージ内にある変数定義を羅列する。
    fn extract_from_package(csttext: &CstText) -> Vec<PackageComponent<InlineCmd>> {
        let cst_stmt_ctx = csttext.cst.pickup(Rule::let_inline_stmt_ctx);
        let cst_stmt_noctx = csttext.cst.pickup(Rule::let_inline_stmt_noctx);
        cst_stmt_ctx
            .into_iter()
            .chain(cst_stmt_noctx)
            .filter(|&cst| {
                if let Some(parent) = InlineCmd::find_parent(csttext, cst) {
                    parent != Rule::module_stmt
                } else {
                    false
                }
            })
            .map(|cst| InlineCmd::new_package_variable(csttext, cst))
            .collect()
    }

    /// パッケージの CST + Text 及び
    /// 対象となる let_stmt の CST を与えて、
    /// パッケージ内にある変数定義を羅列する。
    fn new_package_variable(csttext: &CstText, cst_stmt: &Cst) -> PackageComponent<InlineCmd> {
        let visibility = PackageVisibility::Public;
        let body = InlineCmd::new(csttext, cst_stmt);
        let scope = {
            let start = cst_stmt.span.end;
            let end = csttext.cst.span.end;
            Span { start, end }
        };
        let pos_definition = match cst_stmt.rule {
            Rule::let_inline_stmt_noctx => &cst_stmt.inner[0],
            Rule::let_inline_stmt_ctx => &cst_stmt.inner[1],
            _ => unreachable!(),
        }
        .span
        .start;
        PackageComponent {
            visibility,
            body,
            pos_definition,
            scope,
        }
    }

    /// cst_stmt で定義されるコマンドを返す。
    fn new(csttext: &CstText, cst_stmt: &Cst) -> InlineCmd {
        let cst_cmd_name = match cst_stmt.rule {
            Rule::let_inline_stmt_noctx => &cst_stmt.inner[0],
            Rule::let_inline_stmt_ctx => &cst_stmt.inner[1],
            _ => unreachable!(),
        };
        let name = csttext.get_text(cst_cmd_name).to_owned();
        InlineCmd {
            name,
            type_: None,
            type_declaration: None,
        }
    }

    /// その変数定義 (let_stmt) の親が
    /// - Rule::preamble
    /// - Rule::module_stmt
    /// - Rule::bind_stmt
    /// のいずれであるか判定する。
    fn find_parent(csttext: &CstText, cst: &Cst) -> Option<Rule> {
        csttext.cst.get_parent(cst).map(|cst| cst.rule)
    }
}

#[derive(Debug)]
pub struct BlockCmd {
    /// コマンド名。
    pub name: String,
    /// 型情報。
    pub type_: Option<Vec<String>>,
    /// 型情報の載っている場所。
    pub type_declaration: Option<Span>,
}

impl BlockCmd {
    /// パッケージの CST + Text を与えて、パッケージ内にある変数定義を羅列する。
    fn extract_from_package(csttext: &CstText) -> Vec<PackageComponent<BlockCmd>> {
        let cst_stmt_ctx = csttext.cst.pickup(Rule::let_block_stmt_ctx);
        let cst_stmt_noctx = csttext.cst.pickup(Rule::let_block_stmt_noctx);
        cst_stmt_ctx
            .into_iter()
            .chain(cst_stmt_noctx)
            .filter(|&cst| {
                if let Some(parent) = BlockCmd::find_parent(csttext, cst) {
                    parent != Rule::module_stmt
                } else {
                    false
                }
            })
            .map(|cst| BlockCmd::new_package_variable(csttext, cst))
            .collect()
    }

    /// パッケージの CST + Text 及び
    /// 対象となる let_stmt の CST を与えて、
    /// パッケージ内にある変数定義を羅列する。
    fn new_package_variable(csttext: &CstText, cst_stmt: &Cst) -> PackageComponent<BlockCmd> {
        let visibility = PackageVisibility::Public;
        let body = BlockCmd::new(csttext, cst_stmt);
        let scope = {
            let start = cst_stmt.span.end;
            let end = csttext.cst.span.end;
            Span { start, end }
        };
        let pos_definition = match cst_stmt.rule {
            Rule::let_block_stmt_noctx => &cst_stmt.inner[0],
            Rule::let_block_stmt_ctx => &cst_stmt.inner[1],
            _ => unreachable!(),
        }
        .span
        .start;
        PackageComponent {
            visibility,
            body,
            pos_definition,
            scope,
        }
    }

    /// cst_stmt で定義されるコマンドを返す。
    fn new(csttext: &CstText, cst_stmt: &Cst) -> BlockCmd {
        let cst_cmd_name = match cst_stmt.rule {
            Rule::let_block_stmt_noctx => &cst_stmt.inner[0],
            Rule::let_block_stmt_ctx => &cst_stmt.inner[1],
            _ => unreachable!(),
        };
        let name = csttext.get_text(cst_cmd_name).to_owned();
        BlockCmd {
            name,
            type_: None,
            type_declaration: None,
        }
    }

    /// その変数定義 (let_stmt) の親が
    /// - Rule::preamble
    /// - Rule::module_stmt
    /// - Rule::bind_stmt
    /// のいずれであるか判定する。
    fn find_parent(csttext: &CstText, cst: &Cst) -> Option<Rule> {
        csttext.cst.get_parent(cst).map(|cst| cst.rule)
    }
}

#[derive(Debug)]
pub struct MathCmd {
    /// コマンド名。
    pub name: String,
    /// 型情報。
    pub type_: Option<Vec<String>>,
    /// 型情報の載っている場所。
    pub type_declaration: Option<Span>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageVisibility {
    /// let_stmt などで定義された値。そのpackageを追加すると使用できる類のもの。
    Public,
    /// なにかの式で変数束縛を行う際に定義された一時的な変数。
    Binded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleVisibility {
    /// sig にて direct に宣言されたもの。 Module. が無くとも使うことができる。
    Direct,
    /// sig にて val で宣言されたもの。 Module.* の形で、または open Module すれば使用できる。
    Public,
    /// sig にて宣言されていないもの。 Module の外からは呼び出せない。
    Private,
    /// なにかの式で変数束縛を行う際に定義された一時的な変数。
    Binded,
}
