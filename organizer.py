import multiprocessing
import os
import random
import string
import zipfile
import shutil

def id_generator(size=6, chars=string.ascii_uppercase + string.digits):
    return ''.join(random.choice(chars) for _ in range(size))


def manifest_extract(dirpath,file_name):
    i = 0
    while i < 3:
        try:
            if os.path.isfile(dirpath+"/AndroidManifest.xml"):
                return
            path = os.path.join(dirpath, file_name)
            rnd = id_generator()
            os.system("/opt/jadx/bin/jadx -s "+path+" -d "+rnd)
            os.rename(rnd+"/resources/AndroidManifest.xml", dirpath+"/AndroidManifest.xml")
            os.system("rm -rf "+rnd)
            print("Extracted AndroidManifest.xml from "+path)
        except:
            i += 1
            continue

def main():
    apks = set()
    for target_dirs in [f for f in os.listdir() if os.path.isdir(f)]:
        for d in os.listdir(target_dirs):
            path = target_dirs+"/"+d
            if os.path.isdir(path) and os.path.isfile(path+"/"+d+".apk"):
                shutil.move(path+"/"+d+".apk", path+"/base.apk")
                apks.add((path,"base.apk"))
            elif os.path.isfile(path) and path.endswith(".apk"):
                pkg = ".".join(d.split(".")[:-1])
                new_path = target_dirs+"/"+pkg
                try:
                    os.mkdir(new_path)
                    shutil.move(path, new_path+"/base.apk")
                    apks.add((new_path,"base.apk"))
                except:
                    pass
            elif os.path.isfile(path) and path.endswith(".xapk"):
                pkg = ".".join(d.split(".")[:-1])
                new_path = target_dirs+"/"+pkg
                try:
                    os.mkdir(new_path)
                    with zipfile.ZipFile(path, "r") as zip_ref:
                        zip_ref.extractall(new_path)
                        shutil.move(new_path+"/"+pkg+".apk", new_path+"/base.apk")
                    os.remove(path)
                    apks.add((new_path,"base.apk"))
                except:
                    pass
            elif os.path.isfile(path+"/base.apk"):
                apks.add((path,"base.apk"))
            else:
                pass
    pool = multiprocessing.Pool(multiprocessing.cpu_count())
    pool.starmap(manifest_extract, apks)

if __name__ == '__main__':
    main()
