-- glitu.lua: presents a glitu video (`glitu build video`) click by click in mpv.
--
-- Each chapter of the video is a click of the presentation: it starts with the click's state,
-- frozen (the hold), then plays the animation to the next click. The script pauses on entering
-- each chapter, and maps the keys of glitu-play. It acts only on a video tagged GLITU_PLAY, so it
-- can be installed for all of mpv (~/.config/mpv/scripts/).
--
-- glitu-play runs it for you (`glitu-play talk.mkv`); `glitu-play --mpv-script` prints it.
-- Option: --script-opts=glitu-start=<SLIDE> starts at that slide.

local active = false   -- the file is a glitu video
local pausing = true   -- pause at each click (`c` toggles: continuous play)

local function is_glitu()
    for key in pairs(mp.get_property_native("metadata") or {}) do
        if key:upper() == "GLITU_PLAY" then
            return true
        end
    end
    return false
end

local function chapters()
    return mp.get_property_number("chapters", 0)
end

local function current()
    return mp.get_property_number("chapter", 0)
end

-- The start of chapter `c` (its hold), paused
local function go(c)
    c = math.max(0, math.min(c, chapters() - 1))
    mp.set_property_number("chapter", c)
    mp.set_property_bool("pause", pausing)
end

-- Next: resume to the next click; during an animation, its end at once
local function next_click()
    if mp.get_property_bool("pause") then
        if current() < chapters() - 1 or not mp.get_property_bool("eof-reached") then
            mp.set_property_bool("pause", false)
        end
    else
        go(current() + 1)
    end
end

-- Previous: the click before (during an animation, the click it left)
local function previous_click()
    if mp.get_property_bool("pause") then
        go(current() - 1)
    else
        go(current())
    end
end

local bindings = {
    { { "RIGHT", "DOWN", "SPACE", "ENTER", "KP_ENTER", "PGDWN", "n", "MBTN_LEFT" }, next_click },
    { { "LEFT", "UP", "BS", "PGUP", "p" }, previous_click },
    { { "HOME" }, function() go(0) end },
    { { "END" }, function() go(chapters() - 1) end },
    { { "f", "F11" }, function() mp.command("cycle fullscreen") end },
    { { "ESC" }, function() mp.set_property_bool("fullscreen", false) end },
    { { "q" }, function() mp.command("quit") end },
    { { "c" }, function()
        pausing = not pausing
        mp.osd_message(pausing and "Pause at each click" or "Continuous play")
        if not pausing then
            mp.set_property_bool("pause", false)
        end
    end },
}

local function bind(on)
    for i, b in ipairs(bindings) do
        for _, key in ipairs(b[1]) do
            local name = "glitu-" .. i .. "-" .. key
            if on then
                mp.add_forced_key_binding(key, name, b[2])
            else
                mp.remove_key_binding(name)
            end
        end
    end
end

-- Entering a chapter: pause (the delay falls on the hold, identical frames)
mp.observe_property("chapter", "number", function(_, c)
    if active and pausing and c then
        mp.set_property_bool("pause", true)
    end
end)

mp.register_event("file-loaded", function()
    active = is_glitu()
    bind(active)
    if not active then
        return
    end
    mp.set_property("keep-open", "yes")
    mp.set_property("osd-on-seek", "no")
    local start = tonumber(mp.get_opt("glitu-start") or "")
    if start then
        local list = mp.get_property_native("chapter-list") or {}
        for i, ch in ipairs(list) do
            if (ch.title or ""):match("^" .. start .. "%.") then
                go(i - 1)
                return
            end
        end
    end
    mp.set_property_bool("pause", true)
end)
