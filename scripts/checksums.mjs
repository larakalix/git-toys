import { readdir,readFile,writeFile } from 'node:fs/promises'
import { join,relative } from 'node:path'
import { createHash } from 'node:crypto'
const root=process.argv[2]??'target/release/bundle'
async function walk(path){let result=[];for(const item of await readdir(path,{withFileTypes:true})){const p=join(path,item.name);if(item.isDirectory())result.push(...await walk(p));else if(/\.(dmg|exe|msi|zip|gz)$/.test(item.name))result.push(p)}return result}
const files=(await walk(root)).sort()
await writeFile(join(root,'SHA256SUMS'),(await Promise.all(files.map(async p=>`${createHash('sha256').update(await readFile(p)).digest('hex')}  ${relative(root,p).replaceAll('\\','/')}`))).join('\n')+'\n')
console.log(`Checksummed ${files.length} installers`)
