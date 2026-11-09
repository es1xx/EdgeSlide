-- Model Hyprland 0.57 registration semantics, including errors that do not throw.
local native, calls, errors = {}, {}, {}
hl = {
  gesture=function(spec)
    if spec.action == 'unset' then
      for index,old in ipairs(native) do
        if old.fingers==spec.fingers and old.direction==spec.direction and old.scale==spec.scale then table.remove(native,index); return end
      end
      errors[#errors+1]='not found'; return
    end
    for _,old in ipairs(native) do
      local axis=(spec.direction=='left' or spec.direction=='right') and 'horizontal' or 'vertical'
      if old.fingers==spec.fingers and (old.direction==spec.direction or old.direction==axis or old.direction=='swipe') then errors[#errors+1]='conflict';return end
    end
    native[#native+1]=spec
  end,
  dsp={focus=function(value) return {workspace=value.workspace} end},
  dispatch=function(value) assert(type(value)=='table');calls[#calls+1]=value.workspace end,
  exec_cmd=function(value) calls[#calls+1]=value end
}
local function spec(direction,action)
 return {name='test',fingers=4,direction=direction,enabled=true,action=action or 'next',threshold=80,command=''}
end
local function load(specs) assert(loadfile('data/multifinger.lua'))(specs) end
local function swipe(x,y,options)
 options=options or {};local cb=native[#native].action
 cb.start({time_ms=100,type='swipe',fingers=4,delta={x=0,y=0}})
 cb.update({time_ms=200,type='swipe',fingers=options.fingers or 4,delta={x=x,y=y}})
 cb.finish({time_ms=options.time or 300,type='swipe',cancelled=options.cancelled or false})
 cb.finish({time_ms=400,type='swipe',cancelled=false}) -- repeated end must not dispatch
end
load({spec('right')});swipe(100,0);assert(#calls==1 and calls[1]=='+1')
swipe(20,0);assert(#calls==1,'short motion')
swipe(-100,0);assert(#calls==1,'wrong direction')
swipe(100,100);assert(#calls==1,'diagonal')
swipe(100,0,{cancelled=true});assert(#calls==1,'cancelled')
swipe(100,0,{fingers=3});assert(#calls==1,'finger count changed')
swipe(100,0,{time=4000});assert(#calls==1,'stale gesture')
swipe(100,0);assert(#calls==2,'new contact should work')
load({spec('left','previous')});assert(#native==1,'replacement must not duplicate')
swipe(-100,0);assert(calls[#calls]=='-1')
local sensitive=spec('right');sensitive.threshold=20;load({sensitive});swipe(25,0);assert(calls[#calls]=='+1')
local active=native[1].action;active.start({time_ms=100});active.update({fingers=4,type='swipe',delta={x=100,y=0}})
load({});local count=#calls;active.finish({type='swipe',time_ms=300,cancelled=false});assert(#calls==count,'in-flight old callback cancelled');assert(#native==0)
-- Existing foreign horizontal registration wins. A failed claim must never remove it.
local foreign={fingers=4,direction='horizontal',scale=1,action=function()end}
native[1]=foreign;load({spec('left')});assert(#errors>0 and native[1]==foreign and #native==1)
load({});assert(native[1]==foreign and #native==1,'foreign gesture must survive cleanup')
native={};errors={};rawset(_G,'__edgeslide_multifinger_v1',nil)
local shot=spec('down','screenshot');load({shot});swipe(0,100);assert(calls[#calls]:find('slurp',1,true))
local custom=spec('up','custom');custom.command='literal user command';load({custom});swipe(0,-100);assert(calls[#calls]=='literal user command')
load({});assert(#native==0)
print('Native gesture tests passed: thresholds, directions, cancellation, single firing, replacement, sensitivity, foreign conflicts, commands')
