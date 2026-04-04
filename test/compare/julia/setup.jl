#!/usr/bin/env julia
# Install all Julia dependencies required for the comparison tests.
#
# Usage:
#   julia --project=test/compare/julia test/compare/julia/setup.jl

import Pkg
Pkg.activate(@__DIR__)

println("Installing Julia comparison test dependencies...")

packages = [
    "ROMEO",
    "CLEARSWI",
    "MriResearchTools",
    "NIfTI",
    "ArgParse",
    "JSON3",
]

for pkg in packages
    println("  Adding $pkg...")
    Pkg.add(pkg)
end

println("Precompiling packages...")
Pkg.precompile()

println("✓ Julia setup complete")
println("  Project: ", Base.active_project())
