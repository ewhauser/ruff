use ruff_benchmark::criterion::{
    criterion_group, criterion_main, BenchmarkGroup, BenchmarkId, Criterion, Throughput,
};
use ruff_benchmark::{TestCase, TestFile, TestFileDownloadError};
use ruff_linter::linter::lint_only;
use ruff_linter::settings::rule_table::RuleTable;
use ruff_linter::settings::{flags, LinterSettings};
use ruff_linter::source_kind::SourceKind;
use ruff_linter::{registry::Rule, RuleSelector};
use ruff_python_ast::PySourceType;

#[cfg(target_os = "windows")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(all(
    not(target_os = "windows"),
    not(target_os = "openbsd"),
    any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "powerpc64"
    )
))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

fn create_test_cases() -> Result<Vec<TestCase>, TestFileDownloadError> {
    Ok(vec![
        TestCase::fast(TestFile::try_download("numpy/globals.py", "https://raw.githubusercontent.com/numpy/numpy/89d64415e349ca75a25250f22b874aa16e5c0973/numpy/_globals.py")?),
        TestCase::fast(TestFile::try_download("unicode/pypinyin.py", "https://raw.githubusercontent.com/mozillazg/python-pinyin/9521e47d96e3583a5477f5e43a2e82d513f27a3f/pypinyin/standard.py")?),
        TestCase::normal(TestFile::try_download(
            "pydantic/types.py",
            "https://raw.githubusercontent.com/pydantic/pydantic/83b3c49e99ceb4599d9286a3d793cea44ac36d4b/pydantic/types.py",
        )?),
        TestCase::normal(TestFile::try_download("numpy/ctypeslib.py", "https://raw.githubusercontent.com/numpy/numpy/e42c9503a14d66adfd41356ef5640c6975c45218/numpy/ctypeslib.py")?),
        TestCase::slow(TestFile::try_download(
            "large/dataset.py",
            "https://raw.githubusercontent.com/DHI/mikeio/b7d26418f4db2909b0aa965253dbe83194d7bb5b/tests/test_dataset.py",
        )?),
    ])
}

fn benchmark_linter(mut group: BenchmarkGroup, settings: &LinterSettings) {
    let test_cases = create_test_cases().unwrap();

    for case in test_cases {
        group.throughput(Throughput::Bytes(case.code().len() as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(case.name()),
            &case,
            |b, case| {
                let kind = SourceKind::Python(case.code().to_string());
                b.iter(|| {
                    let path = case.path();
                    let result = lint_only(
                        &path,
                        None,
                        settings,
                        flags::Noqa::Enabled,
                        &kind,
                        PySourceType::from(path.as_path()),
                    );

                    // Assert that file contains no parse errors
                    assert_eq!(result.error, None);
                });
            },
        );
    }

    group.finish();
}

fn benchmark_default_rules(criterion: &mut Criterion) {
    let group = criterion.benchmark_group("linter/default-rules");
    benchmark_linter(group, &LinterSettings::default());
}

fn benchmark_all_rules(criterion: &mut Criterion) {
    let mut rules: RuleTable = RuleSelector::All.all_rules().collect();

    // Disable IO based rules because it is a source of flakiness
    rules.disable(Rule::ShebangMissingExecutableFile);
    rules.disable(Rule::ShebangNotExecutable);

    let settings = LinterSettings {
        rules,
        ..LinterSettings::default()
    };

    let group = criterion.benchmark_group("linter/all-rules");
    benchmark_linter(group, &settings);
}

criterion_group!(default_rules, benchmark_default_rules);
criterion_group!(all_rules, benchmark_all_rules);
criterion_main!(default_rules, all_rules);
