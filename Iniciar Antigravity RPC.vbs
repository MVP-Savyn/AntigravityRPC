Set fso = CreateObject("Scripting.FileSystemObject")
Set shell = CreateObject("WScript.Shell")
baseDir = fso.GetParentFolderName(WScript.ScriptFullName)
mainPy = fso.BuildPath(baseDir, "main.py")
shell.Run "pythonw """ & mainPy & """", 0, False
