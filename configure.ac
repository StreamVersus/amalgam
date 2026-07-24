dnl Process this file with autoconf to produce a configure script.
dnl
dnl   autoconf
dnl   ./configure [options]
dnl
dnl Generates .cargo/config.toml for the amalgam project.
dnl
dnl Usage:  ./configure [--with-android-ndk=DIR] [--with-xwin-dir=DIR]
dnl                    [--without-android] [--without-xwin]
dnl                    [--without-cranelift] [--with-linker=PROG]

AC_INIT([amalgam], [0.1.1], [], [amalgam])
AC_CONFIG_SRCDIR([Cargo.toml])
exec 5>/dev/null
rm -f config.status

dnl ── Options ──────────────────────────────────────────────────────────────
AC_ARG_WITH([android-ndk],
    [AS_HELP_STRING([--with-android-ndk=DIR],
        [path to Android NDK (default: detect from ANDROID_NDK_HOME or well-known paths)])],
    [ANDROID_NDK=$withval])

AC_ARG_WITH([xwin-dir],
    [AS_HELP_STRING([--with-xwin-dir=DIR],
        [path to xwin SDK directory (detected: XWIN_DIR, $HOME/.xwin, $HOME/.cache/cargo-xwin/xwin, ./.xwin-cache/splat)])],
    [XWIN_DIR=$withval])

AC_ARG_WITH([linker],
    [AS_HELP_STRING([--with-linker=PROG],
        [linker program (default: auto-detect wild > mold > lld > clang)])],
    [FORCE_LINKER=$withval])

AC_ARG_WITH([cranelift],
    [AS_HELP_STRING([--without-cranelift],
        [disable cranelift codegen backend even if available)])],
    [],
    [with_cranelift=yes])

AC_ARG_WITH([android],
    [AS_HELP_STRING([--without-android],
        [skip Android NDK configuration)])],
    [],
    [with_android=yes])

AC_ARG_WITH([xwin],
    [AS_HELP_STRING([--without-xwin],
        [skip Windows xwin configuration)])],
    [],
    [with_xwin=yes])

dnl ── Checks for programs ──────────────────────────────────────────────────
AC_CHECK_PROGS([RUSTUP], [rustup])
AS_IF([test -z "$RUSTUP"], [AC_MSG_WARN([rustup not found; install from https://rustup.rs])])

dnl ── Host triple detection ──────────────────────────────────────────────────
AC_MSG_CHECKING([for host triple])
AC_CHECK_PROGS([RUSTC], [rustc])
HOST_TRIPLE=""
AS_IF([test -n "$RUSTC"], [
    HOST_TRIPLE=`$RUSTC -vV 2>/dev/null | sed -n 's/^host: //p'`
])
AS_IF([test -z "$HOST_TRIPLE"], [
    AS_CASE([`uname -s`],
        [Linux*], [HOST_TRIPLE=`uname -m`-unknown-linux-gnu],
        [Darwin*], [HOST_TRIPLE=`uname -m`-apple-darwin],
        [*CYGWIN*|*MINGW*|*MSYS*], [HOST_TRIPLE=`uname -m`-pc-windows-gnu],
        [HOST_TRIPLE=`uname -m`-unknown-`uname -s | tr 'A-Z' 'a-z'`])
])
AC_MSG_RESULT([$HOST_TRIPLE])

dnl ── rust-toolchain.toml ──────────────────────────────────────────────────
AC_MSG_CHECKING([for rust-toolchain.toml])
AS_IF([test -f "$srcdir/rust-toolchain.toml"], [
    RUST_TOOLCHAIN_PINNED=yes
    AC_MSG_RESULT([yes (pinned)])
], [
    RUST_TOOLCHAIN_PINNED=no
    AC_MSG_RESULT([no])
])

dnl ── cranelift codegen backend ─────────────────────────────────────────────
HAVE_CRANELIFT=no
AS_IF([test "$with_cranelift" != "no"], [
    AC_MSG_CHECKING([for cranelift codegen backend])
    AS_IF([test -n "$RUSTUP"], [
        AS_IF(
            [rustup component list --toolchain nightly 2>/dev/null | grep -q "rustc-codegen-cranelift"],
            [HAVE_CRANELIFT=yes; AC_MSG_RESULT([yes])],
            [
                AS_IF(
                    [rustup component add rustc-codegen-cranelift --toolchain nightly 2>/dev/null],
                    [HAVE_CRANELIFT=yes; AC_MSG_RESULT([yes (installed)])],
                    [AC_MSG_RESULT([no])]
                )
            ]
        )
    ], [
        AC_MSG_RESULT([no (rustup missing)])
    ])
])

AC_SUBST([HAVE_CRANELIFT])

dnl ── Linker detection ─────────────────────────────────────────────────────
AC_MSG_CHECKING([for linker])
LINKER="default"
LINKER_ARGS=""
LINKER_DISPLAY="default"
TARGET_FEATURES=""

AS_IF([test -n "$FORCE_LINKER"], [
    LINKER="$FORCE_LINKER"
    LINKER_DISPLAY="$FORCE_LINKER (forced)"
    AC_MSG_RESULT([$LINKER (forced)])
], [
    AC_CHECK_PROGS([WILD], [wild])
    AC_CHECK_PROGS([MOLD], [mold])
    AC_CHECK_PROGS([LLD], [ld.lld lld])

    AS_IF([test -n "$WILD"], [
        LINKER="clang"
        LINKER_ARGS="-C link-arg=--ld-path=wild"
        LINKER_DISPLAY="wild"
        AC_MSG_RESULT([wild])
    ], [test -n "$MOLD"], [
        LINKER="clang"
        LINKER_ARGS="-C link-arg=--ld-path=mold"
        LINKER_DISPLAY="mold"
        AC_MSG_RESULT([mold])
    ], [test -n "$LLD"], [
        LINKER="clang"
        LINKER_ARGS="-C link-arg=--ld-path=$LLD"
        LINKER_DISPLAY="lld"
        AC_MSG_RESULT([lld])
    ], [
        LINKER="clang"
        LINKER_DISPLAY="clang"
        AC_MSG_RESULT([clang])
    ])
])

AC_SUBST([LINKER])
AC_SUBST([LINKER_ARGS])

dnl ── CPU features ─────────────────────────────────────────────────────────
AC_MSG_CHECKING([for CPU features])
AS_CASE([`uname -s`],
    [Linux*], [
        AS_IF([test -r /proc/cpuinfo], [
            feat=""
            grep -q 'avx' /proc/cpuinfo 2>/dev/null && feat="${feat}+avx,"
            grep -q 'fma' /proc/cpuinfo 2>/dev/null && feat="${feat}+fma,"
            grep -q 'sse' /proc/cpuinfo 2>/dev/null && feat="${feat}+sse,"
            TARGET_FEATURES=`echo "$feat" | sed 's/,$//'`
            AS_IF([test -n "$TARGET_FEATURES"], [AC_MSG_RESULT([$TARGET_FEATURES])], [AC_MSG_RESULT([none])])
        ], [AC_MSG_RESULT([none])])
    ],
    [darwin*], [TARGET_FEATURES=""; AC_MSG_RESULT([none])],
    [TARGET_FEATURES=""; AC_MSG_RESULT([none])]
)
AC_SUBST([TARGET_FEATURES])

dnl ── Android NDK detection ────────────────────────────────────────────────
AC_MSG_CHECKING([for Android NDK])
ANDROID_LINKER=""
ANDROID_CC=""
ANDROID_CXX=""
ANDROID_AR=""

AS_IF([test "$with_android" = "no"], [
    AC_MSG_RESULT([disabled])
], [
    AS_IF([test -z "$ANDROID_NDK"], [
        AS_IF([test -n "$ANDROID_NDK_HOME" && test -d "$ANDROID_NDK_HOME"], [
            ANDROID_NDK="$ANDROID_NDK_HOME"
        ], [test -d "$HOME/Android/Sdk/ndk"], [
            ANDROID_NDK=`ls -d "$HOME/Android/Sdk/ndk/"*/ 2>/dev/null | sort -V | tail -1 | sed 's|/$||'`
        ], [test -d /opt/android-ndk], [
            ANDROID_NDK=/opt/android-ndk
        ])
    ])

    AS_IF([test -n "$ANDROID_NDK"], [
        NDK_BIN="$ANDROID_NDK/toolchains/llvm/prebuilt/linux-x86_64/bin"
        AS_IF([test -d "$NDK_BIN"], [
            for api in 35 34 33 32 31 30 29 28 26 24 21; do
                AS_IF([test -f "$NDK_BIN/aarch64-linux-android${api}-clang"], [
                    ANDROID_LINKER="$NDK_BIN/aarch64-linux-android${api}-clang"
                    break
                ])
            done
            AS_IF([test -z "$ANDROID_LINKER"], [
                ANDROID_LINKER=`ls "$NDK_BIN"/aarch64-linux-android*-clang 2>/dev/null | head -1`
            ])
            AS_IF([test -n "$ANDROID_LINKER"], [
                ANDROID_CC="$ANDROID_LINKER"
                ANDROID_CXX=`echo "$ANDROID_LINKER" | sed 's/clang$/clang++/'`
                ANDROID_AR="$NDK_BIN/llvm-ar"
                AC_MSG_RESULT([found: $ANDROID_NDK])
            ], [
                ANDROID_NDK=""
                AC_MSG_WARN([NDK found but no aarch64-linux-android clang])
                AC_MSG_RESULT([not usable])
            ])
        ], [
            dnl try darwin tag
            NDK_BIN="$ANDROID_NDK/toolchains/llvm/prebuilt/darwin-x86_64/bin"
            AS_IF([test -d "$NDK_BIN"], [
                for api in 35 34 33 32 31 30 29 28 26 24 21; do
                    AS_IF([test -f "$NDK_BIN/aarch64-linux-android${api}-clang"], [
                        ANDROID_LINKER="$NDK_BIN/aarch64-linux-android${api}-clang"
                        break
])
                done
                AS_IF([test -z "$ANDROID_LINKER"], [
                    ANDROID_LINKER=`ls "$NDK_BIN"/aarch64-linux-android*-clang 2>/dev/null | head -1`
                ])
                AS_IF([test -n "$ANDROID_LINKER"], [
                    ANDROID_CC="$ANDROID_LINKER"
                    ANDROID_CXX=`echo "$ANDROID_LINKER" | sed 's/clang$/clang++/'`
                    ANDROID_AR="$NDK_BIN/llvm-ar"
                    AC_MSG_RESULT([found: $ANDROID_NDK])
                ], [
                    ANDROID_NDK=""
                    AC_MSG_RESULT([not usable])
                ])
            ], [
                ANDROID_NDK=""
                AC_MSG_RESULT([not found (no toolchain for this host)])
            ])
        ])
    ], [
        AC_MSG_RESULT([not found])
    ])
])
AC_SUBST([ANDROID_LINKER])
AC_SUBST([ANDROID_CC])
AC_SUBST([ANDROID_CXX])
AC_SUBST([ANDROID_AR])

dnl ── Windows xwin detection ───────────────────────────────────────────────
AC_MSG_CHECKING([for xwin (Windows SDK)])

AS_IF([test "$with_xwin" = "no"], [
    XWIN_DIR=""
    AC_MSG_RESULT([disabled])
], [
    AS_IF([test -z "$XWIN_DIR"], [
        AS_IF([test -d "$HOME/.xwin"], [
            XWIN_DIR="$HOME/.xwin"
        ], [test -d "$HOME/.cache/cargo-xwin/xwin"], [
            XWIN_DIR="$HOME/.cache/cargo-xwin/xwin"
        ], [test -d ".xwin-cache/splat"], [
            XWIN_DIR=".xwin-cache/splat"
        ])
    ])

    AS_IF([test -n "$XWIN_DIR"], [
        dnl check for any known arch in crt/lib
        AS_IF([test -d "$XWIN_DIR/crt/lib/x86_64" || test -d "$XWIN_DIR/crt/lib/aarch64" || test -d "$XWIN_DIR/crt/lib/x86" || test -d "$XWIN_DIR/crt/lib/aarch"], [
            AC_MSG_RESULT([found: $XWIN_DIR])
            AC_CHECK_PROGS([CLANG_CL], [clang-cl])
            AC_CHECK_PROGS([LLVM_LIB], [llvm-lib])
            AS_IF([test -z "$CLANG_CL"], [AC_MSG_WARN([clang-cl not found, needed for Windows cross-compilation])])
            AS_IF([test -z "$LLVM_LIB"], [AC_MSG_WARN([llvm-lib not found, needed for Windows cross-compilation])])
        ], [
            XWIN_DIR=""
            AC_MSG_RESULT([incomplete (no CRT libs found)])
        ])
    ], [
        AC_MSG_RESULT([not found])
    ])
])
AC_SUBST([XWIN_DIR])

dnl ── Generate .cargo/config.toml ──────────────────────────────────────────
AS_MKDIR_P([.cargo])

cat > .cargo/config.toml << 'EOF'
# Generated by ./configure
# Re-generate with: ./configure

@<:@unstable@:>@
codegen-backend = true

EOF

dnl host target
cat >> .cargo/config.toml << EOF
@<:@target.$HOST_TRIPLE@:>@
linker = "$LINKER"
EOF

AS_IF([test -n "$LINKER_ARGS" || test -n "$TARGET_FEATURES"], [
    echo "rustflags = @<:@" >> .cargo/config.toml
    AS_IF([test -n "$LINKER_ARGS"], [
        for arg in $LINKER_ARGS; do
            echo "    \"$arg\"," >> .cargo/config.toml
        done
    ])
    AS_IF([test -n "$TARGET_FEATURES"], [
        echo "    \"-C\"," >> .cargo/config.toml
        echo "    \"target-feature=$TARGET_FEATURES\"," >> .cargo/config.toml
    ])
    echo "@:>@" >> .cargo/config.toml
])

echo "" >> .cargo/config.toml

dnl Android target
AS_IF([test -n "$ANDROID_LINKER"], [
    cat >> .cargo/config.toml << EOF
@<:@target.aarch64-linux-android@:>@
linker = "$ANDROID_LINKER"
rustflags = @<:@"-C", "target-feature=+neon"@:>@

EOF
])

dnl WASM targets
cat >> .cargo/config.toml << 'EOF'
@<:@target.wasm32-wasip1@:>@
rustflags = @<:@"-C", "target-feature=+atomics,+bulk-memory"@:>@

@<:@target.wasm32-unknown-unknown@:>@
rustflags = @<:@"-C", "opt-level=z", "--cfg", "web_sys_unstable_apis"@:>@

EOF

dnl Windows MSVC target
AS_IF([test -n "$XWIN_DIR"], [
    cat >> .cargo/config.toml << EOF
@<:@target.x86_64-pc-windows-msvc@:>@
ar = "llvm-lib"
linker = "lld-link"
rustflags = @<:@
    "-Lnative=$XWIN_DIR/crt/lib/x86_64",
    "-Lnative=$XWIN_DIR/sdk/lib/um/x86_64",
    "-Lnative=$XWIN_DIR/sdk/lib/ucrt/x86_64",
@:>@

EOF
])

dnl Profiles
cat >> .cargo/config.toml << 'EOF'
@<:@profile.dev@:>@
incremental = true
EOF

AS_IF([test "$HAVE_CRANELIFT" = "yes"], [
    echo 'codegen-backend = "cranelift"' >> .cargo/config.toml
])

cat >> .cargo/config.toml << 'EOF'
lto = false

@<:@profile.release@:>@
incremental = false
opt-level = 3
lto = "thin"
panic = "abort"
overflow-checks = false
debug-assertions = false
strip = true

EOF


dnl ── Summary ──────────────────────────────────────────────────────────────
cat <<EOF

┌─────────────────────────────────────────────────────────────┐
│                   Configuration Summary                     │
├─────────────────────────────────────────────────────────────┤
EOF

printf "│ %-31s %-27s │\n" "Host target:" "$HOST_TRIPLE"
printf "│ %-31s %-27s │\n" "Linker:" "$LINKER_DISPLAY"
AS_IF([test -n "$TARGET_FEATURES"], [printf "│ %-31s %-27s │\n" "CPU features:" "$TARGET_FEATURES"], [printf "│ %-31s %-27s │\n" "CPU features:" "none"])
AS_IF([test "$HAVE_CRANELIFT" = "yes"], [printf "│ %-31s %-27s │\n" "Cranelift:" "available"], [printf "│ %-31s %-27s │\n" "Cranelift:" "not available"])
AS_IF([test -n "$ANDROID_LINKER"], [printf "│ %-31s %-27s │\n" "Android NDK:" "found"], [printf "│ %-31s %-27s │\n" "Android NDK:" "not found"])
AS_IF([test -n "$XWIN_DIR"], [printf "│ %-31s %-27s │\n" "xwin:" "found"], [printf "│ %-31s %-27s │\n" "xwin:" "not found"])

cat <<EOF
└─────────────────────────────────────────────────────────────┘

Configuration written to .cargo/config.toml
EOF

AC_OUTPUT
rm -f config.log config.status