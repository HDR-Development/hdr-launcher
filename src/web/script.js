const LOCALHOST = "http://localhost";
var AButtonHeld = [false, false, false, false];
var activeMenu = "mainMenu";

function updateBtnDesc(val) {
    document.getElementById("btn-desc").innerHTML = val;
}

function viewProgress() {
    activeMenu = "progress";
    document.getElementById("mainMenu").style.display = 'none';
    document.getElementById("progressSection").style.display = 'flex';
    document.getElementById("progress").style.width = '0%';
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
    // start
    window.nx.sendMessage("start");
    window.location.href = `${LOCALHOST}/start`;
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
        moveUp();
    }

    // Check if D-pad Right pressed or X Axis > 0.7
    if (gamepad.buttons[15].pressed || axisX > 0.7) {
        // Do nothing
    }

    // Check if D-pad Down pressed or Y Axis > 0.7
    if (gamepad.buttons[13].pressed || axisY > 0.7) {
        moveDown();
    };

    //#endregion
}

window.onload = () => {
    Array.from(document.querySelectorAll("#buttons>button")).forEach(item => {
        item.addEventListener("mouseover", () => {
            document.querySelector(".active").classList.remove("active");
            item.classList.add("active");
            updateBtnDesc(item.getAttribute("data-desc") != undefined ? item.getAttribute("data-desc") : item.innerText);
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
        setInterval(function() {
            var gpl = navigator.getGamepads();
            if (gpl.length > 0) {
                for (var i = 0; i < gpl.length; i++) {
                    checkGamepad(i, gpl[i]);
                }
            }
        }, 100);
    });

    window.nx.addEventListener("message", function(e) {
        var info = JSON.parse(e.data);
        updateProgress(info);
        if (info["completed"]) {
            viewMainMenu();
        }
    });

    window.nx.footer.setAssign("B", "", () => {});
    window.nx.footer.setAssign("X", "", () => {});
}