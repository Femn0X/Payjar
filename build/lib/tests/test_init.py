from ui import Window, PJRT  # absolute import works fine in GitHub Actions

def createWindow_test(w, h):
    Window(w, h)

def test_runner(code):
    try:
        PJRT(code)
    except Exception as e:
        print(e)

def test_window_creation():
    createWindow_test(1000, 1000)

def test_runner_failure():
    test_runner("fail")  # should print "PJRT error"
