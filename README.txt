* All transaction types are supported
* Skips transactions with errors
* Spec does not cover some topics, like can "withdrawal" be disputed (assume "no") or should operations for locked account be ignored
* Expects all entries of CSV file to have 4 field, but last one can be empty (means need comma after "tr")
* No storage, so limited by memory size
* For the same reason reads while input file instead of streaming
* Processes in single thread for simplicity and because all operations are artifically fast
* Source code place in single file - easier for limited time/size excercise

# Testing
* "cases" folder has some test cases (just limited by time)
* No tests on wrong "tx" number
* No tests on wrong numbers in "resolve" and "chargeback"
* No tests on accounts serialization logic

# AI usage:
* Code was created in VSCode with Copilot (free tier) enabled
* I've not used Copilot prompts, only used autocompletes
* Copilot predicted logic very well, for example for transaction logic, but had no checks
* When I start to add checks it autocompletes them right too
* Looks like Copilot knows this exercise well, so likely can generate whole solution on such prompt (not tried)
* When adding tests Copilot had more mistakes, but helped in refactoring

