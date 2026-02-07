# Issue 015: Error Reporting Polish

## Status: Planning

## Goal
Production-quality error messages with:
- Real source locations (not `{:?}` spans)
- Source code snippets with underline markers
- One-line "Help:" suggestions for common errors
- Consistent formatting across all error types
- Color-coded output for terminals

## Prerequisites
- None (can run in parallel with 013/014)

## Notes
- Consider `ariadne` or `miette` crate for diagnostic rendering
- Move checker errors already use permission/authority vocabulary — preserve that
- Capability errors already have FsCap/Fs confusion hints — extend this pattern
- Type mismatch errors should show both types with source locations
- Effect mismatch errors should show declared vs actual effect rows

## Design Review: Not yet started
