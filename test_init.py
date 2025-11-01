from .ui import *
def createWindow_test(w,h):
  Window(w,h)
createWindow_test(1000,1000)
def test_runner(code):
  try:PJRT(code)
  except Exception as e:
    print(e)
