# Seq JSON Archive App [![Build status](https://ci.appveyor.com/api/projects/status/3aq07d7prowagwgu?svg=true)](https://ci.appveyor.com/project/datalust/seq-app-jsonarchive) [![NuGet Pre Release](https://img.shields.io/nuget/vpre/Seq.App.JsonArchive.svg)](https://nuget.org/packages/Seq.App.JsonArchive)

Record events from [Seq](https://getseq.net) to a set of newline-delimited JSON streams.

## Getting started

Instructions for installation can be found in the [Seq documentation](https://docs.getseq.net/docs/installing-seq-apps).

**The package id of this package is _Seq.App.JsonArchive_.**

## Reading archived events

Along with regular JSON parsers, the JSON-formatted events written into the archive files can be transformed into [Serilog](https://serilog.net) events using [_Serilog.Formatting.Compact.Reader_](https://github.com/serilog/serilog-formatting-compact-reader), and imported to Seq or interactively queried using [`clef-tool`](https://github.com/datalust/clef-tool).

