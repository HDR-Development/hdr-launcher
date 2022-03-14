const LOCALHOST = "http://localhost";
var AButtonHeld = [false, false, false, false];
var activeMenu = "mainMenu";
var frameDown = [0, 0, 0, 0];
var frameUp = [0, 0, 0, 0];
var counter = 0;

var play_start_sfx;
var play_move_sfx;

function nx_log(message) {
    // if (window.nx != undefined) {
        window.nx.sendMessage("log:" + message);
    // }
}

function updateBtnDesc(val) {
    document.getElementById("btn-desc").innerHTML = val;
}

function updateBtnImg(val) {
    var element = document.getElementById("btn-img");
    element.setAttribute("src", val);
}

function viewProgress() {
    activeMenu = "progress";
    document.getElementById("mainMenu").style.display = 'none';
    document.getElementById("progressSection").style.display = 'flex';
    document.getElementById("progress").style.width = '0%';
}

function viewChangelog(html) {
    activeMenu = "changelog";
    document.getElementById("mainMenu").style.display = 'none';
    document.getElementById("progressSection").style.display = 'none';
    document.getElementById("changelog").innerHTML = html;
    document.getElementById("changelog").style.display = 'block';
}

function viewMainMenu() {
    activeMenu = "mainMenu";
    document.getElementById("mainMenu").style.display = 'block';
    document.getElementById("progressSection").style.display = 'none';
    document.getElementById("progress").style.width = '0%';
}

function updateProgress(info) {
    document.getElementById("progress").style.width = `${info['progress']}%`;
    document.getElementById("progressText").innerHTML = `${info['text']}`;
}

function startHDR() {
    window.nx.sendMessage("start");
}

function versionSelect() {
    // select the version of hdr
    window.nx.sendMessage("version_select");
    viewProgress();
}

function updateHDR() {
    // send session
    window.nx.sendMessage("update_hdr");
    viewProgress();
}

function verifyHDR() {
    // send session
    window.nx.sendMessage("verify_hdr");
    viewProgress();
}

function exit() {
    // quit
    window.nx.sendMessage("exit");
    window.location.href = `${LOCALHOST}/quit`;
}

function moveUp() {
    var source = document.querySelector("#buttons>button.active");
    var target = document.querySelector("#buttons>button.active").previousElementSibling;

    if (source == undefined) {
        target = document.querySelector("#buttons>button:first-child");
    }

    if (target == undefined) {
        target = document.querySelector("#buttons>button:last-child");
    }

    move(source, target);
}

function moveDown() {
    var source = document.querySelector("#buttons>button.active");
    var target = document.querySelector("#buttons>button.active").nextElementSibling;

    if (source == undefined) {
        target = document.querySelector("#buttons>button:first-child");
    }

    if (target == undefined) {
        target = document.querySelector("#buttons>button:first-child");
    }

    move(source, target);
}

function move(source, target) {
    source != undefined ? source.classList.remove("active") : false;
    target != undefined ? target.classList.add("active") : false;
    updateBtnDesc(target != undefined ? target.getAttribute("data-desc") : "No Description Avaliable");
    // updateBtnImg(item.getAttribute("data-img") != undefined ? item.getAttribute("data-img") : item.getAttribute("src"));
    // if (play_move_sfx) {
        play_move_sfx();
    // }
    // cursor_move.play();
}

function click() {
    document.querySelector("#buttons>button.active").click();
}

function checkGamepad(index, gamepad) {
    if (activeMenu == "progress") { return; }

    //#region UI Input Check

    var axisX = gamepad.axes[0];
    var axisY = gamepad.axes[1];

    // Check A button
    if (gamepad.buttons[1].pressed) {
        if (!AButtonHeld[index]) {
            AButtonHeld[index] = true;
            play_start_sfx();
            document.querySelector("#buttons>button.active").click();
        }
    } else {
        AButtonHeld[index] = false;
    }

    // Check if D-pad Left pressed or Left Stick X Axis less than -0.7
    if (gamepad.buttons[14].pressed || axisX < -0.7) {
        // Do nothing
    }

    // Check if D-pad Up pressed or Y-Axis
    if (gamepad.buttons[12].pressed || axisY < -0.7) {
        frameUp[index] = frameUp[index] % 8;
        var should_move = frameUp[index] == 0;
        frameUp[index] += 1;
        if (should_move) {
            moveUp();
        }
    } else {
        frameUp[index] = 0;
    }

    // Check if D-pad Right pressed or X Axis > 0.7
    if (gamepad.buttons[15].pressed || axisX > 0.7) {
        // Do nothing
    }

    // Check if D-pad Down pressed or Y Axis > 0.7
    if (gamepad.buttons[13].pressed || axisY > 0.7) {
        frameDown[index] %= 8;
        var should_move = frameDown[index] == 0;
        frameDown[index] += 1;
        if (should_move) {
            moveDown();
        }
    } else {
        frameDown[index] = 0;
    }

    counter %= 40;
    if (counter == 0) {
        nx_log("wakeup");
    }
    counter++;

    //#endregion
}

window.AudioContext = window.AudioContext || window.webkitAudioContext;

var audioCtx = new window.webkitAudioContext();

window.onload = () => {
    Array.from(document.querySelectorAll("#buttons>button")).forEach(item => {
        item.addEventListener("mouseover", () => {
            document.querySelector(".active").classList.remove("active");
            item.classList.add("active");
            updateBtnDesc(item.getAttribute("data-desc") != undefined ? item.getAttribute("data-desc") : item.innerText);
            updateBtnImg(item.getAttribute("data-img") != undefined ? item.getAttribute("data-img") : item.getAttribute("src"));
            play_move_sfx();
        });
    });

    var activeButton = document.querySelector("#buttons>button.active");
    updateBtnDesc(activeButton.getAttribute("data-desc") != undefined ? activeButton.getAttribute("data-desc") : "No Description Avaliable");

    // Prevent default keydown action
    window.addEventListener('keydown', function(e) {
        e.preventDefault();
    });

    // Listen to the gamepadconnected event
    window.addEventListener("gamepadconnected", function(e) {
        // Once a gamepad has connected, start an interval function that will run every 100ms to check for input
        nx_log("gamepadconnected");
        setInterval(function() {
            var gpl = navigator.getGamepads();
            if (gpl != undefined && gpl.length > 0) {
                for (var i = 0; i < gpl.length; i++) {
                    checkGamepad(i, gpl[i]);
                }
            }
        }, 20);
    });

    window.nx.addEventListener("message", function(e) {
        var info = JSON.parse(e.data);
        if ("contents" in info) {
            if (info["contents"] === "exit") {
                document.location.href = `${LOCALHOST}/start`;
            }
        }
        // document.getElementById("progressSection").innerHTML = info.text;

        // viewChangelog(info["text"]);
        updateProgress(info);
        if (info["completed"]) {
            viewMainMenu();
        }
    });

    window.nx.footer.setAssign("B", "", () => {});
    window.nx.footer.setAssign("X", "", () => {});

    var request = new XMLHttpRequest();
    request.open('GET', './start.wav', true);
    request.responseType = 'arraybuffer';
    request.onload = function () {
        nx_log("request 1 finished");
        nx_log(request.response.byteLength);
        audioCtx.decodeAudioData(request.response, function (buffer) {
            nx_log("setting start");
            play_start_sfx = function() {
                nx_log("playing start");
                var source = audioCtx.createBufferSource();
                source.buffer = buffer;
                source.connect(audioCtx.destination);
                source.start(0);
            };
        }, function (error) {
            nx_log("error:" + error);
        });
    };
    request.send();

    var request2 = new XMLHttpRequest();
    request2.open('GET', 'cursor-move.wav', true);
    request2.responseType = 'arraybuffer';
    request2.onload = function() {
        nx_log("request 2 finished");
        nx_log(request2.response.byteLength);
        audioCtx.decodeAudioData(request2.response, function (buffer) {
            nx_log("bruh");
            play_move_sfx = function() {
                var source = audioCtx.createBufferSource();
                source.buffer = buffer;
                source.connect(audioCtx.destination);
                source.start(0);
                nx_log("move");
            };
        }, function (error) {
            nx_log("error:" + error);
        });
    };
    request2.send();
}