// Copyright (C) 2022-2023 Tanner Reese
/* This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::path::Path;

use afed_objects::testing::run_file;

const BINARY_PATH: &str = "./target/debug/afed";

macro_rules! test_bin {
    ($testname:ident, $filename:literal) => {
        test_bin! {$testname, $filename, []}
    };

    ($testname:ident, $filename:literal, $args:expr) => {
        #[test]
        fn $testname() {
            run_file(BINARY_PATH, Path::new($filename), $args);
        }
    };
}

test_bin! {parse, "parse.af"}
test_bin! {parse_errors, "parse_errors.af"}
test_bin! {help, "help.af"}
test_bin! {func, "func.af"}

test_bin! {use_stmt, "parent_use.af"}
test_bin! {clear, "clear.af", ["-d"]}

test_bin! {object_bool, "object/bool.af"}
test_bin! {object_number, "object/number.af"}
test_bin! {object_string, "object/string.af"}
test_bin! {object_array, "object/array.af"}
test_bin! {object_map, "object/map.af"}

test_bin! {pkgs_math, "pkgs/math.af"}
test_bin! {pkgs_arr, "pkgs/arr.af"}
test_bin! {pkgs_cls, "pkgs/cls.af"}
