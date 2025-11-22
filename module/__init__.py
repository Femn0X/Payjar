from .ui import Window

def createWindow(w, h):
	"""Create and return a Window instance.

	Returning the instance makes the API more useful to callers/tests.
	"""
	return Window(w, h)
