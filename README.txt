* No storage, so limited by memory size
* For the same reason reads while input file instead of streaming
* Expects all entries of CSV file to have 4 field, but last one can be empty (means need comma after "tr")
* Processes in single thread as all operations are artifically fast
* "cases" folder has some test cases (just limited by time to add more)
