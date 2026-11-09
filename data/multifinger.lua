-- EdgeSlide native gestures. No timers, input descriptors, or background process.
-- The unusual scale is an ownership marker for exact-match removal; live callback
-- sensitivity is controlled by our threshold, not Hyprland's animation scale.
local specs = ...
assert(type(hl) == "table" and type(hl.gesture) == "function", "Native Hyprland Lua gestures required")
local marker = 1.000739
local state = rawget(_G, "__edgeslide_multifinger_v1")
if not state then
    state = { registrations = {} }
    rawset(_G, "__edgeslide_multifinger_v1", state)
end
local function request(spec, action)
    hl.gesture({fingers=spec.fingers, direction=spec.direction, mods="", scale=marker, action=action})
end
local function invoke(spec)
    if spec.action == "previous" then
        hl.dispatch(hl.dsp.focus({workspace="-1"}))
    elseif spec.action == "next" then
        hl.dispatch(hl.dsp.focus({workspace="+1"}))
    elseif spec.command ~= "" then
        hl.exec_cmd(spec.command)
    elseif spec.action == "screenshot" then
        -- Cancellation never falls through to a full-screen capture.
        hl.exec_cmd([[sh -c 'area=$(slurp) || exit; [ -n "$area" ] || exit; dir="${XDG_PICTURES_DIR:-$HOME/Pictures}/Screenshots"; mkdir -p "$dir" && grim -g "$area" "$dir/$(date +%Y%m%d-%H%M%S-%N).png"']])
    end
end
local function callbacks(spec)
    local active, x, y, started, invalid = false, 0, 0, 0, false
    return {
        start=function(event)
            active, x, y, started, invalid = true, 0, 0, event.time_ms, false
        end,
        update=function(event)
            if not active then return end
            if event.fingers ~= spec.fingers or event.type ~= "swipe" then invalid=true; return end
            x, y = x + event.delta.x, y + event.delta.y
        end,
        finish=function(event)
            if not active then return end
            active=false -- consume before dispatch: one action at most per contact
            if invalid or event.cancelled or event.type ~= "swipe" then return end
            local elapsed=event.time_ms-started
            if elapsed < 40 or elapsed > 2000 then return end
            local primary, cross
            if spec.direction=="left" then primary,cross=-x,math.abs(y)
            elseif spec.direction=="right" then primary,cross=x,math.abs(y)
            elseif spec.direction=="up" then primary,cross=-y,math.abs(x)
            else primary,cross=y,math.abs(x) end
            if primary >= spec.threshold and primary >= cross*1.5 then invoke(spec) end
        end
    }
end
-- Invalidate callbacks from a gesture in flight before replacing our registrations.
for _,old in pairs(state.registrations) do
    old.action.finish({type="swipe",cancelled=true,time_ms=0})
    request(old.spec,"unset") -- exact marker; never unset a foreign/default-scale binding
end
state.registrations={}
for _,spec in ipairs(specs) do
    if spec.enabled then
        local action=callbacks(spec)
        request(spec,action) -- Hyprland rejects overlaps instead of replacing them
        state.registrations[spec.name]={spec=spec,action=action}
    end
end
