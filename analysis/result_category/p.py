import re

files = ["all.txt","ART_AND_DESIGN.txt","AUTO_AND_VEHICLES.txt","BEAUTY.txt","BOOKS_AND_REFERENCE.txt","BUSINESS.txt","COMICS.txt","COMMUNICATION.txt","DATING.txt","EDUCATION.txt","ENTERTAINMENT.txt","EVENTS.txt","FINANCE.txt","FOOD_AND_DRINK.txt","HEALTH_AND_FITNESS.txt","HOUSE_AND_HOME.txt","LIBRARIES_AND_DEMO.txt","LIFESTYLE.txt","MAPS_AND_NAVIGATION.txt","MEDICAL.txt","MUSIC_AND_AUDIO.txt","NEWS_AND_MAGAZINES.txt","PARENTING.txt","PERSONALIZATION.txt","PHOTOGRAPHY.txt","PRODUCTIVITY.txt","SHOPPING.txt","SOCIAL.txt","SPORTS.txt","TOOLS.txt","TRAVEL_AND_LOCAL.txt","VIDEO_PLAYERS.txt","WEATHER.txt","any.txt"]

for i,file in enumerate(files):
    with open(file, 'r') as f:
        for line in f:
            #leak visible = Network: 12
            # extract 12
            leak = re.findall(r'leak visible = Device: (\d+)', line)
            if leak:
                print("vec![{}, {}, {}],".format(24, i, leak[0]))
                continue
            leak = re.findall(r'leak visible = Network: (\d+)', line)
            if leak:
                print("vec![{}, {}, {}],".format(23, i, leak[0]))
                continue
            leak = re.findall(r'leak visible = Network Location: (\d+)', line)
            if leak:
                print("vec![{}, {}, {}],".format(22, i, leak[0]))
                continue
            leak = re.findall(r'leak visible = GPS: (\d+)', line)
            if leak:
                print("vec![{}, {}, {}],".format(21, i, leak[0]))
                continue
            leak = re.findall(r'leak visible = User Assets: (\d+)', line)
            if leak:
                print("vec![{}, {}, {}],".format(20, i, leak[0]))
                continue
            
            leak = re.findall(r'leak invisible = Device: (\d+)', line)
            if leak:
                print("vec![{}, {}, {}],".format(19, i, leak[0]))
                continue
            leak = re.findall(r'leak invisible = Network: (\d+)', line)
            if leak:
                print("vec![{}, {}, {}],".format(18, i, leak[0]))
                continue
            leak = re.findall(r'leak invisible = Network Location: (\d+)', line)
            if leak:
                print("vec![{}, {}, {}],".format(17, i, leak[0]))
                continue
            leak = re.findall(r'leak invisible = GPS: (\d+)', line)
            if leak:
                print("vec![{}, {}, {}],".format(16, i, leak[0]))
                continue
            leak = re.findall(r'leak invisible = User Assets: (\d+)', line)
            if leak:
                print("vec![{}, {}, {}],".format(15, i, leak[0]))
                continue
            

            leak = re.findall(r'leak works = Device: (\d+)', line)
            if leak:
                print("vec![{}, {}, {}],".format(14, i, leak[0]))
                continue
            leak = re.findall(r'leak works = Network: (\d+)', line)
            if leak:
                print("vec![{}, {}, {}],".format(13, i, leak[0]))
                continue
            leak = re.findall(r'leak works = Network Location: (\d+)', line)
            if leak:
                print("vec![{}, {}, {}],".format(12, i, leak[0]))
                continue
            leak = re.findall(r'leak works = GPS: (\d+)', line)
            if leak:
                print("vec![{}, {}, {}],".format(11, i, leak[0]))
                continue
            leak = re.findall(r'leak works = User Assets: (\d+)', line)
            if leak:
                print("vec![{}, {}, {}],".format(10, i, leak[0]))
                continue
            

            leak = re.findall(r'leak works invisible = Device: (\d+)', line)
            if leak:
                print("vec![{}, {}, {}],".format(9, i, leak[0]))
                continue
            leak = re.findall(r'leak works invisible = Network: (\d+)', line)
            if leak:
                print("vec![{}, {}, {}],".format(8, i, leak[0]))
                continue
            leak = re.findall(r'leak works invisible = Network Location: (\d+)', line)
            if leak:
                print("vec![{}, {}, {}],".format(7, i, leak[0]))
                continue
            leak = re.findall(r'leak works invisible = GPS: (\d+)', line)
            if leak:
                print("vec![{}, {}, {}],".format(6, i, leak[0]))
                continue
            leak = re.findall(r'leak works invisible = User Assets: (\d+)', line)
            if leak:
                print("vec![{}, {}, {}],".format(5, i, leak[0]))
                continue
            

            leak = re.findall(r'leak any = Device: (\d+)', line)
            if leak:
                print("vec![{}, {}, {}],".format(4, i, leak[0]))
                continue
            leak = re.findall(r'leak any = Network: (\d+)', line)
            if leak:
                print("vec![{}, {}, {}],".format(3, i, leak[0]))
                continue
            leak = re.findall(r'leak any = Network Location: (\d+)', line)
            if leak:
                print("vec![{}, {}, {}],".format(2, i, leak[0]))
                continue
            leak = re.findall(r'leak any = GPS: (\d+)', line)
            if leak:
                print("vec![{}, {}, {}],".format(1, i, leak[0]))
                continue
            leak = re.findall(r'leak any = User Assets: (\d+)', line)
            if leak:
                print("vec![{}, {}, {}],".format(0, i, leak[0]))
                continue
            
