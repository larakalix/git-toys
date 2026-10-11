// Isolated test bridge for the actual Rust engine. Never imported by Vite or shipped.
import { createServer } from 'node:http'
import { spawn, execFileSync } from 'node:child_process'
import { mkdtempSync, writeFileSync, mkdirSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join,resolve } from 'node:path'
import { createInterface } from 'node:readline'
const sandbox=mkdtempSync(join(tmpdir(),'git-toys-ui-'))
const source=join(sandbox,'source'),remote=join(sandbox,'origin.git'),checkout=join(sandbox,'checkout')
mkdirSync(source)
const git=(cwd,args)=>execFileSync('git',args,{cwd,encoding:'utf8',env:{...process.env,GIT_CONFIG_GLOBAL:process.platform==='win32'?'NUL':'/dev/null',GIT_CONFIG_NOSYSTEM:'1'},stdio:['ignore','pipe','pipe']}).trim()
git(source,['init','-b','main']);git(source,['config','user.name','Git Toys Fixture']);git(source,['config','user.email','fixture@example.invalid']);git(source,['config','commit.gpgsign','false'])
writeFileSync(join(source,'hello.txt'),'hello\n');git(source,['add','.']);git(source,['commit','-m','Start the project'])
git(source,['branch','conflict-side']);writeFileSync(join(source,'hello.txt'),'main version\n');git(source,['add','.']);git(source,['commit','-m','Update the greeting'])
for(let i=0;i<7;i++){writeFileSync(join(source,`module-${i}.ts`),`export const value = ${i}\n`);git(source,['add','.']);git(source,['commit','-m',`Add project module ${i+1}`])}
git(source,['switch','conflict-side']);writeFileSync(join(source,'hello.txt'),'side version\n');git(source,['add','.']);git(source,['commit','-m','Alternative greeting']);git(source,['switch','main']);git(sandbox,['clone','--bare',source,remote])
const executable=resolve(`target/debug/examples/test_driver${process.platform==='win32'?'.exe':''}`)
const driver=spawn(executable,[sandbox],{stdio:['pipe','pipe','inherit']})
const pending=[];createInterface({input:driver.stdout}).on('line',line=>pending.shift()?.(JSON.parse(line)))
let chain=Promise.resolve();function request(command,args){const task=()=>new Promise(resolve=>{pending.push(resolve);driver.stdin.write(JSON.stringify({command,args})+'\n')});const result=chain.then(task);chain=result.then(()=>{});return result}
let preferences={recent:[],favorites:[],theme:'light',colorTheme:'teal',sidebarWidth:240,detailWidth:440,journal:[]}
createServer(async(req,res)=>{
res.setHeader('Access-Control-Allow-Origin','http://127.0.0.1:1420');res.setHeader('Access-Control-Allow-Headers','Content-Type');res.setHeader('Content-Type','application/json')
if(req.method==='OPTIONS'){res.end();return}
if(req.url==='/fixture'){res.end(JSON.stringify({sandbox,source,remote,checkout}));return}
if(req.url==='/verify'){res.end(JSON.stringify({local:git(checkout,['rev-parse','HEAD']),remote:git(remote,['rev-parse','refs/heads/main'])}));return}
let raw='';for await(const chunk of req)raw+=chunk
try{const {command,args}=JSON.parse(raw);let reply;if(command==='load_preferences')reply={result:preferences};else if(command==='save_preferences'){preferences={...args.preferences,journal:preferences.journal};reply={result:null}}else{reply=await request(command,args);if(command==='mutate'){preferences.journal.push({id:String(Date.now()),repoId:args.id,kind:args.action.kind,status:reply.error?'failed':'completed',timestamp:Date.now()/1000})}}res.end(JSON.stringify(reply))}catch(error){res.statusCode=500;res.end(JSON.stringify({error:{code:'test',message:String(error),detail:''}}))}
}).listen(1421,'127.0.0.1',()=>console.log('Git Toys test bridge ready'))
function cleanup(){driver.kill();rmSync(sandbox,{recursive:true,force:true});process.exit()}
process.on('SIGTERM',cleanup);process.on('SIGINT',cleanup)
