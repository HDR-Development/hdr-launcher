const LOCALHOST = "http://localhost";
var AButtonHeld = [false, false, false, false];
var activeMenu = "mainMenu";
var frameDown = [0, 0, 0, 0];
var frameUp = [0, 0, 0, 0];
var counter = 0;

var play_start_sfx;
var play_move_sfx;
var play_failure;

var nightlies_check = false;
var skip_on_launch_check = false;

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

function viewOptions() {
    activeMenu = "options";
    document.getElementById("mainMenu").style.display = 'none';
    document.getElementById("optionsMenu").style.display = 'block';
    document.getElementById("title").innerHTML = "HDR Launcher > Options";
}

function viewChangelog() {
    activeMenu = "changelog";
    document.getElementById("mainMenu").style.display = 'none';
    document.getElementById("progressSection").style.display = 'none';
    document.getElementById("changelog").style.display = 'block';
    document.getElementById("title").innerHTML = "HDR Launcher > Changelog";
    window.nx.footer.setAssign("B", "", () => { viewMainMenu(); play_move_sfx(); });
    window.nx.footer.setAssign("A", "", () => { viewMainMenu(); play_move_sfx(); });
}

function viewMainMenu() {
    activeMenu = "mainMenu";
    document.getElementById("mainMenu").style.display = 'block';
    document.getElementById("progressSection").style.display = 'none';
    document.getElementById("changelog").style.display = 'none';
    document.getElementById("progress").style.width = '0%';
    document.getElementById("optionsMenu").style.display = 'none';
    document.getElementById("title").innerHTML = "HDR Launcher";
    window.nx.footer.setAssign("B", "", () => { play_failure(); });
}

function toggleNightlies() {
    window.nx.sendMessage("toggle:nightlies");
}

function toggleSkipOnLaunch() {
    window.nx.sendMessage("toggle:skip_on_launch");
}

function updateProgress(info) {
    document.getElementById("progress").style.width = `${info['progress']}%`;
    document.getElementById("progressText").innerHTML = `${info['text']}`;
}

function startHDR() {
    if (document.getElementById("play-button").innerHTML.includes("Restart")) {
        window.nx.sendMessage("restart");
        return;
    }
    window.nx.sendMessage("start");
    document.getElementById("title").style.display = "hidden";
    document.getElementById("mainMenu").style.display = "hidden";
}

function openArcropolis() {
    window.nx.sendMessage("open_arcropolis");
    document.getElementById("title").style.display = "hidden";
    document.getElementById("mainMenu").style.display = "hidden";
}

function startGame() {
    window.location.href = `${LOCALHOST}/start`;
}

function restartGame() {
    window.location.href = `${LOCALHOST}/restart`;
}

function chooseOptions() {
    // select the version of hdr
    window.nx.sendMessage("choose_options");
    var target = document.querySelector("#optionsMenu>button:first-child");
    moveOptions(document.querySelector("#optionsMenu>button.option-active"), target);
    viewOptions();
    window.nx.footer.setAssign("B", "", () => { viewMainMenu(); play_move_sfx(); });
}

function updateHDR() {
    // send session
    var button_html = document.getElementById("update-button").innerHTML;
    if (button_html.includes("Fix") || button_html.includes("Install")) {
        window.nx.sendMessage("reinstall_hdr");
    } else {
        window.nx.sendMessage("update_hdr");
    }
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

function moveUpOptions() {
    var source = document.querySelector("#optionsMenu>button.option-active");
    var target = document.querySelector("#optionsMenu>button.option-active").previousElementSibling;

    if (source == undefined) {
        target = document.querySelector("#optionsMenu>button:first-child");
    }

    if (target == undefined) {
        target = document.querySelector("#optionsMenu>button:first-child");
    }

    moveOptions(source, target);
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

function moveDownOptions() {
    var source = document.querySelector("#optionsMenu>button.option-active");
    var target = document.querySelector("#optionsMenu>button.option-active").nextElementSibling;

    if (source == undefined) {
        target = document.querySelector("#optionsMenu>button:first-child");
    }

    if (target == undefined) {
        target = document.querySelector("#optionsMenu>button:first-child");
    }

    moveOptions(source, target);
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

function moveOptions(source, target) {
    source != undefined ? source.classList.remove("option-active") : false;
    target != undefined ? target.classList.add("option-active") : false;
    // updateBtnImg(item.getAttribute("data-img") != undefined ? item.getAttribute("data-img") : item.getAttribute("src"));
    // if (play_move_sfx) {
        play_move_sfx();
    // }
    // cursor_move.play();
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

function checkGamepadOptions(index, gamepad) {

    var axisX = gamepad.axes[0];
    var axisY = gamepad.axes[1];

    // Check A button
    if (gamepad.buttons[1].pressed) {
        if (!AButtonHeld[index]) {
            AButtonHeld[index] = true;
            play_start_sfx();
            document.querySelector("#optionsMenu>button.option-active").click();
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
            moveUpOptions();
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
            moveDownOptions();
        }
    } else {
        frameDown[index] = 0;
    }
}

function checkGamepad(index, gamepad) {

    counter %= 40;
    if (counter == 0) {
        nx_log("wakeup");
    }
    counter++;
    if (activeMenu == "progress") { return; }
    if (activeMenu == "options") {
        checkGamepadOptions(index, gamepad);
        return;
    }

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
            if (activeMenu === "changelog") {
                window.scrollBy(0, -50);
            } else {
                moveUp();
            }
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
            if (activeMenu === "changelog") {
                window.scrollBy(0, 50);
            } else {
                moveDown();
            }
        }
    } else {
        frameDown[index] = 0;
    }

    //#endregion
}

window.AudioContext = window.AudioContext || window.webkitAudioContext;

var audioCtx = new window.webkitAudioContext();

function formatBPS(bps) {
    var suffix = "bps";
    if (bps > (1024 * 1024)) {
        bps /= (1024 * 1024);
        suffix = "mbps";
    } else if (bps > 1024) {
        bps /= 1024;
        suffix = "kbps";
    }
    return bps.toFixed(2) + " " + suffix;
}

function updateProgressByDownload(download_info) {
    if (download_info["tag"] !== "download-update") return;

    var bps = formatBPS(download_info["bps"]);
    var progress = download_info["bytes_downloaded"] / download_info["total_bytes"];
    if (download_info["total_bytes"] === 0)
        progress = 0.0;
    var progress_bar = document.getElementById("progress");
    progress_bar.style.backgroundColor = "var(--main-progress-download-color)";
    progress_bar.style.width = `${progress * 100}%`;
    document.getElementById("progressText").innerHTML = `Downloading ${download_info["item_name"]}... ${bps}<br>${(progress * 100).toFixed(2)}%`;
}

function updateProgressByExtraction(extract_info) {
    if (extract_info["tag"] !== "extract-update") return;


    document.getElementById("progressParent").style.backgroundColor = "var(--main-progress-download-color)";
    var progress = (extract_info["file_number"] + 1) / extract_info["file_count"];
    var progress_bar = document.getElementById("progress");
    progress_bar.style.width = `${progress * 100}%`;
    progress_bar.style.backgroundColor = "var(--main-button-bg-hover-color)";
    document.getElementById("progressText").innerHTML = `Extracting...<br>${extract_info["file_name"]}<br>${extract_info["file_number"] + 1} / ${extract_info["file_count"]}`;
}

function updateProgressByVerify(extract_info) {
    if (extract_info["tag"] !== "verify-install") return;

    document.getElementById("progressParent").style.backgroundColor = "var(--main-progress-download-color)";
    var progress = (extract_info["file_number"] + 1) / extract_info["file_count"];
    var progress_bar = document.getElementById("progress");
    progress_bar.style.width = `${progress * 100}%`;
    progress_bar.style.backgroundColor = "var(--main-button-bg-hover-color)";
    document.getElementById("progressText").innerHTML = `Verifying...<br>${extract_info["file_name"]}<br>${extract_info["file_number"] + 1} / ${extract_info["file_count"]}`;
}

function changeMenuByCommand(change_menu) {
    if (change_menu["tag"] !== "change-menu") return;

    if (change_menu["going_to"] === "main-menu") {
        viewMainMenu();
    } else if (change_menu["going_to"] === "text-view") {
        viewChangelog();
    } else if (change_menu["going_to"] === "progress") {
        viewProgress();
    }
}

function setOption(set_option) {
    if (set_option["tag"] !== "set-option") return;

    if (set_option["option"] === "nightlies") {
        nx_log(set_option["status"]);
        nightlies_check = set_option["status"] == true;
    } else if (set_option["option"] === "skip_on_launch") {
        skip_on_launch = set_option["status"] == true;
    }

    // if (nightlies_check) {
        document.getElementById("enable_nightlies").style.visibility = nightlies_check == true ? "visible" : "hidden";
        document.getElementById("skip_on_launch").style.visibility = skip_on_launch == false ? "visible" : "hidden";
    // }
}

function changeHtml(change_html) {
    if (change_html["tag"] !== "change-html") return;

    nx_log(change_html["text"]);

    document.getElementById(change_html["id"]).innerHTML = change_html["text"];
}

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
        if (!("tag" in info)) {
            return;
        }


        if (info["tag"] === "download-update") {
            updateProgressByDownload(info);
        } else if (info["tag"] === "versioning") {
            var versioning_string = `HDR | Code: ${info["code"]} | Assets: ${info["romfs"]}`;
            document.getElementById("title").innerHTML = versioning_string;
        } else if (info["tag"] === "extract-update") {
            activeMenu = "progress";
            updateProgressByExtraction(info);
        } else if (info["tag"] === "verify-install") {
            activeMenu = "progress";
            updateProgressByVerify(info);
        } else if (info["tag"] === "start-game") {
            startGame();
        } else if (info["tag"] === "restart-game") {
            restartGame();   
        } else if (info["tag"] === "exit-launcher") {
            exit();
        } else if (info["tag"] === "change-menu") {
            changeMenuByCommand(info);
        } else if (info["tag"] === "change-html") {
            changeHtml(info);
        } else if (info["tag"] === "set-option") {
            setOption(info);
        }
        // document.getElementById("progressSection").innerHTML = info.text;

        // viewChangelog(info["text"]);
        // updateProgress(info);
        if (info["completed"]) {
            viewMainMenu();
        }
    });

    window.nx.sendMessage("load");

    window.nx.footer.setAssign("B", "", () => { play_failure(); });
    window.nx.footer.setAssign("X", "", () => {});

    var request = new XMLHttpRequest();
    request.open('GET', './start.wav', true);
    request.responseType = 'arraybuffer';
    request.onload = function () {
        audioCtx.decodeAudioData(request.response, function (buffer) {
            play_start_sfx = function() {
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
        audioCtx.decodeAudioData(request2.response, function (buffer) {
            play_move_sfx = function() {
                var source = audioCtx.createBufferSource();
                source.buffer = buffer;
                source.connect(audioCtx.destination);
                source.start(0);
            };
        }, function (error) {
            nx_log("error:" + error);
        });
    };
    request2.send();

    var request3 = new XMLHttpRequest();
    request3.open('GET', 'failure.wav', true);
    request3.responseType = 'arraybuffer';
    request3.onload = function() {
        audioCtx.decodeAudioData(request3.response, function (buffer) {
            play_failure = function() {
                var source = audioCtx.createBufferSource();
                source.buffer = buffer;
                source.connect(audioCtx.destination);
                source.start(0);
            };
        }, function (error) {
            nx_log("error:" + error);
        });
    };
    request3.send();
}