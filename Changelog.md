# Changelog

## unreleased

### User Visible

* Keep `has_made_first_move` in local storage so that the UI controls are
  immediately visible if the user loads the application a second time.
* Keep `wants_to_download_solver` in local storage so that the solver download
  starts automatically if the user opens the solver menu and they have already
  downloaded it in a previous session.

### Internal

* Move `ProgressBar` rendering to `components` module and rename it to `Timeline`.

## 2025-12-19

### Internal

* Unify `Jump` and `Move` datatypes
* Unify `Solvability` enum types

## 2025-12-14

* Initial release of webapp & report
