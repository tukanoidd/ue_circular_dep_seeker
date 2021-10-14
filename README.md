I created this for work, to be able to find the circular dependency in modified Unreal Engine codebase. I work on Linux, so this was designed to work with Unreals CMake projects only and I don't know if pathing will work on Windows but I guess you can try it out and see if it's useful for you or not.

Command Line Parameters (all have to be set, I'm too lazy to automate stuff for that):
~p={project_path} (has to be absolute path)
~e={entry_point} (has to be absolute path)
~o={output_file} (has to be absolute path)

Feel free to contribute to expand and optimize this if you want, I'll merge the changes when I can.
