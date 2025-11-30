* Code is in one file - easier for limited time excercise, for longer use should be split
* No storage, so limited by memory size
* For the same reason reads while input file instead of streaming
* Expects all entries of CSV file to have 4 field, but last one can be empty (means need comma after "tr")
* Processes in single thread as all operations are artifically fast

# Testing
* "cases" folder has some test cases (just limited by time to add more)
* Test suite allows to run single case and all cases
* There are no tests on serialization logic

# AI usage:
* Code was created in VSCode with Copilot (free tier) enabled
* I've not used Copilot promts, only used autocomletes
* Copilot predicted well logic I want to write (transactions), but it had very little checks in it.
* When I start to add checks it autocompletes right
* Looks like Copilot knows this excercise well, so likely can generate whole solution on such prompt
* When adding tests Copilot had more mistakes, but helped in refactoring

