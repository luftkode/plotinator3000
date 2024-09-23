cargo_binaries := join(home_directory(), ".cargo/bin/")

# Sets how the common cargo commands are invoked, if 'cargo-limit' is installed then the l-prefixed version of cargo check/run/etc is used, otherwise the default version is.
check := if path_exists(join(cargo_binaries, "cargo-lcheck")) == "true" { "lcheck" } else { "check" }
clippy := if path_exists(join(cargo_binaries, "cargo-lclippy")) == "true" { "lclippy" } else { "clippy" }
run := if path_exists(join(cargo_binaries, "cargo-lrun")) == "true" { "lrun" } else { "run" }
build := if path_exists(join(cargo_binaries, "cargo-lbuild")) == "true" { "lbuild" } else { "build" }
doc := if path_exists(join(cargo_binaries, "cargo-ldoc")) == "true" { "ldoc" } else { "doc" }


# if nextest is installed, nextest is used, otherwise ltest if cargo-limit is installed, otherwise default.
USING_NEX_TEST := if path_exists(join(cargo_binaries, "cargo-nextest")) == "true" { "true" } else { "false" }
USING_LTEST := if USING_NEX_TEST == "true" { 
	"false" 
} else if path_exists(join(cargo_binaries, "cargo-ltest")) == "true" { 
	"true" 
} else { "false" }
test := if USING_NEX_TEST == "true" {
	"nextest run"
} else if USING_LTEST == "true" {
	"ltest"
} else {
	"test"
}