const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    const lexe_dep = b.dependency("lexe", .{
        .target = target,
        .optimize = optimize,
    });
    const lexe_mod = lexe_dep.module("lexe");

    const hwi_dep = b.dependency("hwi", .{
        .target = target,
        .optimize = optimize,
    });
    const hwi_mod = hwi_dep.module("hwi");

    // Main CLI executable
    const exe_mod = b.createModule(.{
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
        .imports = &.{
            .{ .name = "lexe", .module = lexe_mod },
            .{ .name = "hwi", .module = hwi_mod },
        },
    });

    const exe = b.addExecutable(.{
        .name = "oceanln",
        .root_module = exe_mod,
    });
    b.installArtifact(exe);

    const run_cmd = b.addRunArtifact(exe);
    run_cmd.step.dependOn(b.getInstallStep());
    if (b.args) |args| {
        run_cmd.addArgs(args);
    }

    const run_step = b.step("run", "Run the oceanln CLI");
    run_step.dependOn(&run_cmd.step);

    // Unit tests
    const test_mod = b.createModule(.{
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
        .imports = &.{
            .{ .name = "lexe", .module = lexe_mod },
            .{ .name = "hwi", .module = hwi_mod },
        },
    });

    const unit_tests = b.addTest(.{
        .root_module = test_mod,
    });
    const run_unit_tests = b.addRunArtifact(unit_tests);

    const test_step = b.step("test", "Run unit tests");
    test_step.dependOn(&run_unit_tests.step);
}
